//! 一轮纠错闭环：拉日志与学习数据 → 三种修正 → 回放门槛 → 推送 → 写报告。

use std::fmt::Write as _;

use qingjian_cloud_client::{Client, Snapshot};
use qingjian_cloud_proto::{MAX_LEARNING_PUSH, MAX_PAGE};

use crate::args::Args;
use crate::engine_cli::{EngineCli, ReplayScore, offline_config};
use crate::error::TunerError;
use crate::llm::Llm;
use crate::logs;
use crate::proposal::{Origin, Proposal, to_push};
use crate::state::TunerState;
use crate::steps;

/// 回放门槛的结论。
#[derive(Debug, Clone, PartialEq)]
enum Verdict {
    Passed {
        before: ReplayScore,
        after: ReplayScore,
    },
    PassedWithoutTopics {
        before: ReplayScore,
        after: ReplayScore,
    },
    Failed {
        before: ReplayScore,
        after: ReplayScore,
    },
    NotEnoughData {
        holdout: usize,
    },
    NothingToDo,
}

pub fn run_once(args: &Args) -> Result<String, TunerError> {
    let client = Client::new(&args.server, &args.token);
    let me = client.whoami()?;
    let state_path = args.state.join("tuner-state.json");
    let saved_state = TunerState::load(&state_path);
    let mut state = saved_state.clone();
    let llm = Llm::new(&args.server, &args.token, &args.model);

    let lines = logs::fetch(&client)?;
    let snapshot = Snapshot::from_rows(&fetch_learning(&client)?);
    let work = tempfile::tempdir()?;
    let config = match client.config()? {
        Some(doc) => {
            let path = work.path().join("config.toml");
            std::fs::write(&path, offline_config(&doc.text))?;
            Some(path)
        }
        None => None,
    };
    let cli = EngineCli::new(args.cli.clone(), args.data.clone(), config);
    let before_dir = work.path().join("before");
    snapshot.write_ime_dir(&before_dir)?;

    let (train, holdout) = logs::split(&lines, args.train_ratio);
    let mut proposals = steps::audit(&llm, &snapshot, &mut state, args.max_audit)?;
    let compositions = logs::compositions(train);
    proposals.extend(steps::from_compositions(
        &llm,
        &cli,
        &before_dir,
        &compositions,
        &snapshot,
        &mut state,
        args.max_compositions,
    )?);
    let recent = logs::recent_text(train, 3000);
    proposals.extend(steps::topics(
        &llm,
        &cli,
        &before_dir,
        &recent,
        &snapshot,
        &mut state,
        args.max_topics,
    )?);

    let holdout_path = work.path().join("holdout.jsonl");
    let holdout_commits = logs::write_jsonl(holdout, &holdout_path)?;
    let (verdict, accepted) = gate(
        args,
        &cli,
        &snapshot,
        &before_dir,
        &holdout_path,
        holdout_commits,
        &proposals,
    )?;

    let applied = !args.dry_run && !accepted.is_empty();
    if applied {
        push_all(&client, &accepted)?;
    }
    // 门槛没过时，这一轮问过的词下次还要再问（数据多了可能就过了），只留体检记录
    if !args.dry_run {
        if matches!(verdict, Verdict::Failed { .. }) {
            state.seen = saved_state.seen;
        }
        state.save(&state_path)?;
    }

    let report = report(
        &me.device,
        lines.len(),
        &snapshot,
        &proposals,
        &accepted,
        &verdict,
        args.dry_run,
        applied,
    );
    let reports = args.state.join("reports");
    std::fs::create_dir_all(&reports)?;
    std::fs::write(reports.join(format!("{}.md", stamp())), &report)?;
    std::fs::write(reports.join("latest.md"), &report)?;
    Ok(report)
}

fn fetch_learning(client: &Client) -> Result<Vec<qingjian_cloud_proto::LearningRow>, TunerError> {
    let mut rows = Vec::new();
    let mut since = 0;
    loop {
        let page = client.learning(since, MAX_PAGE)?;
        let full = page.rows.len() == MAX_PAGE;
        if let Some(last) = page.rows.last() {
            since = last.seq;
        }
        rows.extend(page.rows);
        if !full {
            return Ok(rows);
        }
    }
}

/// 留出段够大就回放比较；变差先去掉话题补词再比一次，还变差就全不要。留出段太小就只要有用户证据的修正。
fn gate(
    args: &Args,
    cli: &EngineCli,
    snapshot: &Snapshot,
    before_dir: &std::path::Path,
    holdout: &std::path::Path,
    holdout_commits: usize,
    proposals: &[Proposal],
) -> Result<(Verdict, Vec<Proposal>), TunerError> {
    if proposals.is_empty() {
        return Ok((Verdict::NothingToDo, Vec::new()));
    }
    if holdout_commits < args.min_holdout {
        let evidenced: Vec<Proposal> = proposals
            .iter()
            .filter(|p| p.origin() != Origin::Topic)
            .cloned()
            .collect();
        return Ok((
            Verdict::NotEnoughData {
                holdout: holdout_commits,
            },
            evidenced,
        ));
    }
    let before = cli.replay(holdout, before_dir)?;
    let score = |set: &[Proposal]| -> Result<ReplayScore, TunerError> {
        let mut after = snapshot.clone();
        after.apply_push(&to_push(set));
        let dir = tempfile::tempdir()?;
        after.write_ime_dir(dir.path())?;
        cli.replay(holdout, dir.path())
    };
    let after = score(proposals)?;
    if after.hits >= before.hits {
        return Ok((Verdict::Passed { before, after }, proposals.to_vec()));
    }
    let without_topics: Vec<Proposal> = proposals
        .iter()
        .filter(|p| p.origin() != Origin::Topic)
        .cloned()
        .collect();
    if without_topics.len() < proposals.len() {
        let after = score(&without_topics)?;
        if after.hits >= before.hits {
            return Ok((
                Verdict::PassedWithoutTopics { before, after },
                without_topics,
            ));
        }
    }
    Ok((Verdict::Failed { before, after }, Vec::new()))
}

fn push_all(client: &Client, accepted: &[Proposal]) -> Result<(), TunerError> {
    for chunk in accepted.chunks(MAX_LEARNING_PUSH / 2) {
        client.push_learning(&to_push(chunk))?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn report(
    device: &str,
    log_lines: usize,
    snapshot: &Snapshot,
    proposals: &[Proposal],
    accepted: &[Proposal],
    verdict: &Verdict,
    dry_run: bool,
    applied: bool,
) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "# 青简 Cloud 纠错闭环 {}\n", stamp());
    let words = snapshot
        .set_entries(qingjian_cloud_client::Table::Words)
        .count();
    let _ = writeln!(
        out,
        "以「{device}」身份运行；日志 {log_lines} 行，用户词 {words} 个。\n"
    );
    let rate = |s: &ReplayScore| format!("{:.1}%（{} / {}）", s.rate() * 100.0, s.hits, s.total);
    let line = match verdict {
        Verdict::Passed { before, after } => {
            format!("回放门槛通过：首选命中 {} → {}", rate(before), rate(after))
        }
        Verdict::PassedWithoutTopics { before, after } => format!(
            "带话题补词时回放变差，去掉后通过：首选命中 {} → {}",
            rate(before),
            rate(after)
        ),
        Verdict::Failed { before, after } => format!(
            "回放门槛没过，这一轮什么都不改：首选命中 {} → {}",
            rate(before),
            rate(after)
        ),
        Verdict::NotEnoughData { holdout } => format!(
            "留出段只有 {holdout} 次上屏，不够做回放门槛：只应用有用户实际证据的修正，不做话题补词"
        ),
        Verdict::NothingToDo => "没有要改的。".to_owned(),
    };
    let _ = writeln!(out, "{line}\n");
    let status = if dry_run {
        "试运行，没有推送。"
    } else if applied {
        "已推送到服务器，各设备下次同步学习数据时生效。"
    } else {
        "没有推送。"
    };
    let _ = writeln!(out, "{status}\n");
    let _ = writeln!(out, "## 采纳（{}）\n", accepted.len());
    for p in accepted {
        let _ = writeln!(out, "- {}", p.describe());
    }
    let rejected: Vec<&Proposal> = proposals.iter().filter(|p| !accepted.contains(p)).collect();
    if !rejected.is_empty() {
        let _ = writeln!(out, "\n## 未采纳（{}）\n", rejected.len());
        for p in rejected {
            let _ = writeln!(out, "- {}", p.describe());
        }
    }
    out
}

/// `2026-10-03T110500Z` 这样的 UTC 时间戳，不为此引入时间库。
fn stamp() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default();
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}{:02}{:02}Z",
        rem / 3600,
        rem % 3600 / 60,
        rem % 60
    )
}
