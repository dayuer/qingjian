# 上下文预测实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 候选按宿主前文排序（「汽车」后 `youxiang` 出 油箱、「我现在」后出 又想），再加本地 Tab 续写；只在 `sujian` 分支。

**Architecture:** Core 加最小挂钩（前文 → `Context`、choice 改加分、词级重排与续写各一个入口），新逻辑放新文件；
`qingjian-neural` 加贪心续写；macOS 壳同时加载通变与知微、把续写挂到现有整句补全的位置；iOS 只接前文。
每处上游文件改动记进 `cloud/docs/fork-patch.md`。

**Tech Stack:** Rust 1.96（qingjian-core、qingjian-neural、qingjian-platform、apps/cli、apps/macos、cloud/crates/qingjian-cloud-bridge）。

**Spec:** [`cloud/docs/specs/2026-10-04-context-prediction-design.md`](../specs/2026-10-04-context-prediction-design.md)。

**审计（2026-10-04，adc825f）定下的事**，本版已照改：整句不参与词级重排；评测用产品配置（通变 + 知微）跑、只用通变作对照；
CLI 报按键同步部分 p50 / p99；后台线程打分与续写各留最新一条；热切 `tongbian` 真卸知微；私密输入不读前文；
β 以 `--replay` 不降为准；e2e 先预置前文；续写 p90 > 150 ms 停下报告；Tab 行为变化写 `cloud/docs/design.md`；
续写上屏记 `InputSource::LocalContinuation`、首选照常记 choice。

---

## 与大纲的两处出入（审计已同意）

1. **评测集放 `cloud/data/eval/context-pairs.tsv`，不放 `data/eval/`。** 根 `.gitignore` 第 7 行是 `/data`，整个目录不入库。
2. **`--eval-continuation` 在 Task 6 第一步。** 它量的是 `continue_text`，Task 6 才有。

## 代码约定（每个任务都照办）

- 新文件都有 `//!` 文件头；新文件不用 `use super::*`（文件内的 `#[cfg(test)] mod tests` 除外），写精确路径。
- 测试先写、先跑一次确认它失败，再实现。
- 依赖随包模型的测试：模型不在就 `eprintln!("没有知微模型，跳过")` 并 `return`，在本文件「评测记录」备注里写明哪些测试因此跳过了。
- 提交信息按 Conventional Commits，**不加 AI 署名**；**不跳过** `.githooks/pre-commit`（文档提交也走钩子）。
- 每处上游文件改动，同一提交里记 `cloud/docs/fork-patch.md`。

## 工作环境

工作区 `/Users/liyuqing/sproot/qingjian-context-prediction`（分支 `feat/context-prediction`，从 `sujian` 开，已 cherry-pick `b7485e3`）。
产品数据与模型在 `qingjian-mainline/data/`（gitignore），链进来：

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && ln -s /Users/liyuqing/sproot/qingjian-mainline/data data && ls data/generated data/models/hanzhang-zhiwei data/models/hanzhang-tongbian
```

### 评测命令

CLI 缺省读用户的 `config.toml`，里面云联想开着、密钥在输入法进程的 .env 里，CLI 拿不到就退出；评测用一份关掉云联想的副本
（其余项与用户一致：模糊音、方案、词库开关）：

```bash
python3 -c "import re,os;s=open(os.path.expanduser('~/Library/Application Support/Qingjian/config.toml'),encoding='utf-8').read();m=re.search(r'(?ms)^\[predict\]\n.*?(?=^\[|\Z)',s);sec=re.sub(r'(?m)^enabled\s*=\s*true','enabled = false',m.group(0));open('/tmp/eval-config.toml','w',encoding='utf-8').write(s[:m.start()]+sec+s[m.end():])"
```

两套配置都跑：**产品配置**（素笺缺省 `scorers = "both"`：通变排整句 + 知微排词；Task 4 之前还没有 `--word-model`，就只带通变）
与**对照**（只用通变 = 上游行为）。门槛按产品配置的数字算，对照一起报。

产品配置（Task 4 起加 `--word-model data/models/hanzhang-zhiwei`，之前去掉这一段；zsh 不按空格拆变量，参数要写全）：

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && for e in "--eval-context cloud/data/eval/context-pairs.tsv" "--eval-text data/eval/sentences.tsv --misses 0" "--replay data/eval/input-log-2026-10-04.jsonl --misses 0"; do eval cargo run --release -q -p qingjian-cli -- --config /tmp/eval-config.toml --neural data/models/hanzhang-tongbian --neural-async --word-model data/models/hanzhang-zhiwei $e; done
```

对照（只用通变）：

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && for e in "--eval-context cloud/data/eval/context-pairs.tsv" "--eval-text data/eval/sentences.tsv --misses 0" "--replay data/eval/input-log-2026-10-04.jsonl --misses 0"; do eval cargo run --release -q -p qingjian-cli -- --config /tmp/eval-config.toml --neural data/models/hanzhang-tongbian --neural-async $e; done
```

回放用的输入日志是活的（输入法一直在写），Task 1 冻结了一份到 `data/eval/input-log-2026-10-04.jsonl`（gitignore 的 data/ 下，只在本机），
之后每个任务都比这一份。

模型都用 `--neural-async`（壳里的接法）：报告里「按键同步部分」只算第一次 `query()`，模型在后台、`settle` 等它回来再查一次；
同步接法会把模型几十毫秒算进 p99，与壳不符。`--replay` 现在也像 `--eval-text` 一样等异步重排（Task 1 加的）。

门槛（产品配置）：`--eval-context` 有前文首选比基线 **≥ +15 个百分点**；`--eval-text` 首选、`--replay` 「词」首选比基线**下降不超过 0.5 个百分点**；
`--replay` 与 `--eval-context` 报的**按键同步部分 p99** 不超过基线 +1 ms。任何一项越线：停下，把数字发给审计会话「素笺输入法」，不往下做。

每个任务完成的检查：

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && cargo fmt --all && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
```

## 评测记录

每格写「产品配置 / 对照」两个数。

| 任务 | `--eval-context` 有前文 / 无前文 | `--eval-text` 首选 / 字准确率 | `--replay` 词 / 整句 首选 | 同步 p50 / p99 ms（replay） | 备注 |
|---|---|---|---|---|---|
| 基线（Task 1，2026-10-04，只有通变） | 40.3% / 40.3% | 37.2% / 78.9% | 87.7% / 72.7% | eval-context 0.1 / 0.6；replay 0.3 / 1.8 | 404 对；期望不在候选 0；连跑两次一致 |
| Task 2 前文进词级排序 | | | | | |
| Task 3 choice 改加分 β= | | | | | |
| Task 4 知微词级重排 λ_w= | | | | | |
| Task 5 `tongbian` 对照 | 与对照列逐字相同 | | | | 内存：both / tongbian MB（真人量） |
| Task 6 续写 τ= | 显示率 / 代理精度 / p50 / p90 | | | | |
| Task 7 真机 | TextEdit 显示 / 接受 __ / 10；聊天应用 __ / 10（真人） | | | | |

---

## Task 1：评测集、`--eval-context`、按键同步 p50 / p99

**Files:**
- Create: `cloud/data/eval/context-pairs.tsv`
- Create: `apps/cli/src/latency.rs`（分位数）
- Create: `apps/cli/src/eval/context.rs`
- Modify: `apps/cli/src/main.rs:6-15`（`mod latency;`）、`:93`（分派）
- Modify: `apps/cli/src/eval/mod.rs:9-15`（`pub mod context;`）
- Modify: `apps/cli/src/args.rs`（两个参数）
- Modify: `apps/cli/src/replay/mod.rs:137-144`、`apps/cli/src/replay/report.rs`（记每次查询的同步耗时，报 p50 / p99）
- Modify: `cloud/docs/fork-patch.md`（新小节）

- [ ] **Step 1：写评测集**

格式：`拼音<TAB>前文<TAB>期望`，`#` 开头是注释，同一拼音至少两条、前文不同、期望不同；前文是一两句自然的中文，
以汉字结尾（标点结尾的那组用来测「标点视为句首」，单独放最后）。≥300 对，人工写、不含真实用户数据。
按同音词组写，每组 2–3 条，每条前文 4–12 个字。开头示例（照这个口气写满）：

```tsv
# 同拼音不同上文的词级评测：拼音	前文	期望首选。人工整理，不含真实用户数据。
youxiang	汽车	油箱
youxiang	请把文件发到我的	邮箱
youxiang	我现在	又想
gongshi	这道题要代入	公式
gongshi	他出去办	公事
gongshi	对方发起了	攻势
shiyan	我们做一个	实验
shiyan	这是他的	誓言
shiyan	对方	食言
jiaoshi	她是一名	教师
jiaoshi	请到三楼的	教室
yanjiu	他喜欢喝	烟酒
yanjiu	我们要认真	研究
xingshi	案件的	形式
xingshi	事态的	形势
xingshi	这是	刑事
tongzhi	已经下发	通知
tongzhi	各位	同志
zhiliao	他到医院	治疗
zhiliao	这是重要的	资料
shiji	二十一	世纪
shiji	他的	事迹
shiji	抓住	时机
shiji	这不切合	实际
```

分组方向（每组写 2–3 条，凑到 300 对以上）：职业 / 地点（教师 教室、医生 医师）、动作 / 名词（治疗 资料、研究 烟酒）、
抽象 / 具体（形式 形势 刑事、公式 公事 攻势）、量词与数字（世纪 事迹 时机 实际）、常见单字（吧 把、在 再、的 地 得、他 她 它）、
动补与否定（有点 又点、不是 步时）。

- [ ] **Step 2：核对评测集格式**

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && awk -F'\t' '!/^#/ && NF==3 {n++; seen[$1]++} END {print n " 对"; for (k in seen) if (seen[k] < 2) print "只有一条前文: " k}' cloud/data/eval/context-pairs.tsv
```

Expected: `≥300 对`，没有「只有一条前文」的行。

- [ ] **Step 3：写 `apps/cli/src/latency.rs`**

```rust
//! 耗时分位数：回放与评测报告「按键同步部分」的 p50 / p99 用。

use std::time::Duration;

/// 一组耗时，攒着算分位数。
#[derive(Debug, Default, Clone)]
pub struct Latencies(Vec<Duration>);

impl Latencies {
    pub fn push(&mut self, elapsed: Duration) {
        self.0.push(elapsed);
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// `quantile` 在 0 到 1 之间；空集返回 0。最近秩法：排好序取第 ⌈q·n⌉ 个。
    pub fn quantile(&self, quantile: f64) -> Duration {
        if self.0.is_empty() {
            return Duration::ZERO;
        }
        let mut sorted = self.0.clone();
        sorted.sort();
        let rank = ((quantile * sorted.len() as f64).ceil() as usize).clamp(1, sorted.len());
        sorted[rank - 1]
    }

    /// 报告里的一句：`p50 0.8 ms / p99 3.1 ms`。
    pub fn summary(&self) -> String {
        format!(
            "p50 {:.1} ms / p99 {:.1} ms",
            self.quantile(0.5).as_secs_f64() * 1000.0,
            self.quantile(0.99).as_secs_f64() * 1000.0,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantiles_use_nearest_rank() {
        let mut latencies = Latencies::default();
        for ms in [5, 1, 3, 2, 4] {
            latencies.push(Duration::from_millis(ms));
        }
        assert_eq!(latencies.quantile(0.5), Duration::from_millis(3));
        assert_eq!(latencies.quantile(0.99), Duration::from_millis(5));
        assert_eq!(latencies.quantile(0.0), Duration::from_millis(1));
        assert_eq!(Latencies::default().quantile(0.5), Duration::ZERO);
    }
}
```

`main.rs` 的 `mod eval;` 后加 `mod latency;`。

- [ ] **Step 4：写 `apps/cli/src/eval/context.rs`**

```rust
//! 同拼音不同上文的词级评测：`拼音\t前文\t期望` 三列，逐行冷启动、写入前文、查拼音，看首选是不是期望的词；
//! 同一份数据再跑一遍不给前文，两个数字的差就是上文带来的提升。前文走 `Engine::history_mut()`，
//! 与 `--eval-text` 一样（引擎没有壳给的前文时就用本会话历史，见 `Engine::rescoring_context`）。
//! 另报按键同步部分（第一次 `query()`，不含等后台模型）的 p50 / p99。

use std::fmt;
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use qingjian_core::Engine;

use crate::eval::EvalError;
use crate::latency::Latencies;

/// 一条评测对。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextPair {
    pub pinyin: String,

    /// 光标前的文字；空串就是句首。
    pub before: String,

    /// 期望的首选。
    pub expected: String,
}

impl ContextPair {
    /// 解析一行；空行、`#` 开头的注释、列数不对的行返回 `None`。
    pub fn parse(line: &str) -> Option<Self> {
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() || line.starts_with('#') {
            return None;
        }
        let mut columns = line.split('\t');
        let pinyin = columns.next()?.trim();
        let before = columns.next()?.trim();
        let expected = columns.next()?.trim();
        (!pinyin.is_empty() && !expected.is_empty()).then(|| Self {
            pinyin: pinyin.to_owned(),
            before: before.to_owned(),
            expected: expected.to_owned(),
        })
    }
}

/// 一对在一种设置下的结果。
#[derive(Debug, Clone, Default)]
struct Outcome {
    /// 期望在候选里的名次（0 是首选）；`None` 是不在候选里。
    position: Option<usize>,

    /// 拼音切不动。
    unparsable: bool,

    top: Vec<String>,

    /// 第一次 `query()` 的耗时：按键回调里同步做的那部分。
    sync: Duration,

    /// 含等后台模型、再查一次的总耗时。
    total: Duration,
}

/// 评测报告。
#[derive(Debug, Default)]
pub struct ContextReport {
    pub total: usize,

    /// 拼音切不动的对数（两种设置下一样）。
    pub unparsable: usize,

    /// 有前文时首选命中。
    pub with_context: usize,

    /// 不给前文时首选命中。
    pub without_context: usize,

    /// 有前文时期望根本不在候选里：多半是评测集的拼音或期望写错了。
    pub missing: usize,

    /// 有前文那一遍的同步耗时与总耗时。
    pub sync: Latencies,
    pub total_time: Duration,

    /// 有前文仍没命中首选的例子。
    pub misses: Vec<String>,
}

impl ContextReport {
    pub fn evaluated(&self) -> usize {
        self.total - self.unparsable
    }

    fn percent(part: usize, whole: usize) -> f64 {
        if whole == 0 {
            0.0
        } else {
            part as f64 * 100.0 / whole as f64
        }
    }
}

impl fmt::Display for ContextReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let evaluated = self.evaluated();
        let with = Self::percent(self.with_context, evaluated);
        let without = Self::percent(self.without_context, evaluated);
        writeln!(f, "同拼音不同上文评测（冷启动，不学习）")?;
        writeln!(
            f,
            "对数 {:>5}  有前文首选 {with:>5.1}%  无前文首选 {without:>5.1}%  提升 {:+.1} 个百分点  期望不在候选 {}",
            self.total,
            with - without,
            self.missing,
        )?;
        if evaluated > 0 {
            writeln!(
                f,
                "按键同步部分 {}，含等模型的平均 {:.1} ms",
                self.sync.summary(),
                self.total_time.as_secs_f64() * 1000.0 / evaluated as f64,
            )?;
        }
        if self.unparsable > 0 {
            writeln!(f, "其中 {} 对拼音切不动", self.unparsable)?;
        }
        if !self.misses.is_empty() {
            writeln!(f, "\n有前文仍没命中首选的例子：")?;
            for miss in &self.misses {
                writeln!(f, "  {miss}")?;
            }
        }
        Ok(())
    }
}

/// 读评测集。
pub fn load(path: &Path) -> Result<Vec<ContextPair>, EvalError> {
    let text = std::fs::read_to_string(path).map_err(|source| EvalError::Read {
        path: path.to_owned(),
        source,
    })?;
    Ok(text.lines().filter_map(ContextPair::parse).collect())
}

/// 跑一遍：每对先给前文查一次，再不给前文查一次。`details` 给了就逐对写 JSONL。
pub fn run(
    engine: &mut Engine,
    path: &Path,
    show_misses: usize,
    details: Option<&Path>,
) -> Result<ContextReport, EvalError> {
    let pairs = load(path)?;
    let mut writer = details
        .map(|path| {
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
                .map(BufWriter::new)
                .map_err(|source| EvalError::Write {
                    path: path.to_owned(),
                    source,
                })
        })
        .transpose()?;
    let mut report = ContextReport::default();
    for pair in &pairs {
        report.total += 1;
        let with = evaluate(engine, pair, Some(&pair.before));
        let without = evaluate(engine, pair, None);
        if with.unparsable {
            report.unparsable += 1;
            continue;
        }
        report.sync.push(with.sync);
        report.total_time += with.total;
        report.with_context += usize::from(with.position == Some(0));
        report.without_context += usize::from(without.position == Some(0));
        report.missing += usize::from(with.position.is_none());
        if with.position != Some(0) && report.misses.len() < show_misses {
            report.misses.push(format!(
                "{:<14} {:<16} 期望 {:<6} 有前文前三 {}{}",
                pair.pinyin,
                pair.before,
                pair.expected,
                with.top.join(" / "),
                with.position
                    .map_or(String::from("（不在候选里）"), |i| format!("（第 {} 位）", i + 1)),
            ));
        }
        if let Some(writer) = &mut writer {
            let row = serde_json::json!({
                "pinyin": pair.pinyin, "before": pair.before, "expected": pair.expected,
                "position_with_context": with.position, "top_with_context": with.top,
                "position_without_context": without.position, "top_without_context": without.top,
                "sync_ms": with.sync.as_secs_f64() * 1000.0,
                "total_ms": with.total.as_secs_f64() * 1000.0,
            });
            writeln!(writer, "{row}").map_err(|source| EvalError::Write {
                path: details.expect("writer has path").to_owned(),
                source,
            })?;
        }
    }
    if let Some(writer) = &mut writer {
        writer.flush().map_err(|source| EvalError::Write {
            path: details.expect("writer has path").to_owned(),
            source,
        })?;
    }
    Ok(report)
}

/// 评一对：清空引擎、写（或不写）前文、喂拼音、看期望在第几位。异步重打分像壳一样请求并等结果。
fn evaluate(engine: &mut Engine, pair: &ContextPair, before: Option<&str>) -> Outcome {
    engine.clear();
    engine.break_chain();
    engine.history_mut().clear();
    if let Some(before) = before {
        engine.history_mut().record(before);
    }
    engine.set_input(&pair.pinyin);
    let started = Instant::now();
    let Ok(query) = engine.query() else {
        engine.clear();
        return Outcome {
            unparsable: true,
            ..Outcome::default()
        };
    };
    let sync = started.elapsed();
    let query = if crate::rescoring::settle(engine) {
        engine.query().unwrap_or(query)
    } else {
        query
    };
    let total = started.elapsed();
    let items = &query.candidates.items;
    let outcome = Outcome {
        position: items.iter().position(|c| c.text == pair.expected),
        unparsable: false,
        top: items.iter().take(3).map(|c| c.text.clone()).collect(),
        sync,
        total,
    };
    engine.clear();
    outcome
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_three_columns_and_skips_comments() {
        assert_eq!(ContextPair::parse("# 注释"), None);
        assert_eq!(ContextPair::parse(""), None);
        assert_eq!(ContextPair::parse("youxiang\t汽车"), None);
        assert_eq!(
            ContextPair::parse("youxiang\t汽车\t油箱\r"),
            Some(ContextPair {
                pinyin: "youxiang".into(),
                before: "汽车".into(),
                expected: "油箱".into(),
            })
        );
        // 前文可以为空（句首那组）
        assert_eq!(ContextPair::parse("ba\t\t把").unwrap().before, "");
    }
}
```

- [ ] **Step 5：挂进 `eval/mod.rs`、`args.rs`、`main.rs`**

`apps/cli/src/eval/mod.rs` 第 9 行后加：

```rust
pub mod context;
```

`apps/cli/src/args.rs` 在 `eval_details` 字段后加：

```rust
    /// 同拼音不同上文评测：读 `拼音\t前文\t期望` 三列（cloud/data/eval/context-pairs.tsv），
    /// 每对先给前文、再不给前文各查一次，报告两种设置下的首选命中率与按键同步部分的 p50 / p99
    #[arg(long)]
    pub eval_context: Option<PathBuf>,

    /// 把 --eval-context 的逐对结果写成 JSONL（拒绝覆盖）
    #[arg(long, requires = "eval_context")]
    pub eval_context_details: Option<PathBuf>,
```

`apps/cli/src/main.rs` 在 `if !args.eval_text.is_empty() {` 之前加：

```rust
    if let Some(path) = &args.eval_context {
        let report = eval::context::run(
            &mut engine,
            path,
            args.misses,
            args.eval_context_details.as_deref(),
        )?;
        print!("{report}");
        return Ok(());
    }
```

- [ ] **Step 6：`--replay` 报同步 p50 / p99**

`apps/cli/src/replay/report.rs`：`use crate::latency::Latencies;`，`Report` 加字段：

```rust
    /// 每次上屏重查时 `query()` 的耗时（按键回调里同步做的那部分，不含等后台模型）。
    pub sync: Latencies,
```

`Display` 里 `write_tally` 调用之后（第一个 `writeln!` 前后都行）加：

```rust
        if !self.sync.is_empty() {
            writeln!(f, "按键同步部分 {}", self.sync.summary())?;
        }
```

`apps/cli/src/replay/mod.rs` 的 `replay_commit` 里，`let query = match engine.query() {` 改成：

```rust
    let started = std::time::Instant::now();
    let queried = engine.query();
    report.sync.push(started.elapsed());
    let query = match queried {
```

- [ ] **Step 7：编译、跑单测**

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && cargo test -p qingjian-cli
```

Expected: 全绿，含 `parses_three_columns_and_skips_comments`、`quantiles_use_nearest_rank`。

- [ ] **Step 8：跑基线，连跑两次 `--eval-context` 确认数字一致**

「评测命令」里**对照**那条（此时产品配置也只有通变）跑一次，`--eval-context` 再跑一次看是否一致。看报告里的「期望不在候选」：
逐条检查（`--eval-context-details /tmp/ctx.jsonl` 看 `top_with_context`），是评测集写错的改评测集，重跑。数字填进「评测记录」基线行。

- [ ] **Step 9：记 fork-patch、提交**

`cloud/docs/fork-patch.md` 在「合并上游时」小节之前加：

```markdown
### 上下文预测（素笺，设计见 cloud/docs/specs/2026-10-04-context-prediction-design.md）

评测集 `cloud/data/eval/context-pairs.tsv`（同拼音不同上文），命令 `qingjian-cli --eval-context`；计划与评测记录在 `cloud/docs/plans/2026-10-04-context-prediction.md`。

| 文件 | 改动 | 说明 |
|---|---|---|
| `apps/cli/src/latency.rs` | 新文件 | 耗时分位数 |
| `apps/cli/src/eval/context.rs` | 新文件 | `--eval-context` 的实现与报告 |
| `apps/cli/src/eval/mod.rs` | 加 1 行 | `pub mod context;` |
| `apps/cli/src/args.rs` | 加 2 个参数 | `--eval-context`、`--eval-context-details` |
| `apps/cli/src/main.rs` | 加 1 行 mod、1 个分支 | 分派 |
| `apps/cli/src/replay/mod.rs`、`report.rs` | 加 3 行、1 个字段 | 回放报按键同步 p50 / p99 |
```

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && cargo fmt --all && git add cloud/data/eval/context-pairs.tsv apps/cli cloud/docs/fork-patch.md cloud/docs/plans/2026-10-04-context-prediction.md && git commit -m "feat(cli): 同拼音不同上文的词级评测 --eval-context，回放报按键同步 p99

- 300+ 对人工整理的 拼音/前文/期望，同一拼音至少两种前文
- 每对有前文、无前文各查一次，差值就是上文带来的提升
- 评测集放 cloud/data/eval/（根 .gitignore 不收 data/）"
```

---

## Task 2：宿主前文进词级排序与整句首词

**Files:**
- Create: `crates/qingjian-core/src/engine/query/left_context.rs`
- Create: `crates/qingjian-core/src/engine/tests/context_fork.rs`
- Modify: `crates/qingjian-core/src/engine/query/mod.rs:5-13`（`mod left_context;`）
- Modify: `crates/qingjian-core/src/engine/query/phonetic.rs:145-164`
- Modify: `crates/qingjian-core/src/engine/query/converting.rs:205-215`
- Modify: `crates/qingjian-core/src/sentence/viterbi.rs:52-125`（`convert_paths` 加 `start` 参数）、`:348-374`（`best_predecessor`）
- Modify: `crates/qingjian-core/src/engine/tests/mod.rs:3-17`（`mod context_fork;`）
- Modify: `apps/macos/src/imk/controller/display.rs:11-25`（第一键总是读前文，私密不读）
- Modify: `cloud/docs/fork-patch.md`

现状：词级排序只认 `self.chain.context()`（本会话上一个上屏的词），整句 Viterbi 第一个词固定 `Context::START`；
壳给的光标前文（`Engine::rescoring_context()`：壳给了就是应用里的文字，没给就是本会话历史，截 64 字）只给神经重排看。
已有 `sentence::segment_text` 能按静态模型把汉字切成词，直接拿来切前文末尾。私密输入（`Engine::is_private`）时不看前文。

- [ ] **Step 1：写失败的单测 `crates/qingjian-core/src/engine/tests/context_fork.rs`**

```rust
//! 宿主前文进词级排序与整句首词（素笺分叉，见 cloud/docs/specs/2026-10-04-context-prediction-design.md）。

use qingjian_dictionary::Dictionary;

use crate::candidate::CandidateKind;
use crate::engine::Engine;
use crate::engine::query::left_context::{LeftContext, left_context_of};
use crate::sentence::{Context, LanguageModel, NoLanguageModel};

/// 汽车 → 油箱、发送 → 邮箱；句首 邮箱 比 油箱 略常见。
struct ContextModel;

impl LanguageModel for ContextModel {
    fn log_prob(&self, previous: Option<&str>, word: &str) -> Option<f64> {
        Some(match (previous, word) {
            (Some("汽车"), "油箱") => -2.0,
            (Some("汽车"), "邮箱") => -9.0,
            (Some("发送"), "邮箱") => -2.0,
            (Some("发送"), "油箱") => -9.0,
            (_, "邮箱") => -5.0,
            (_, "油箱") => -5.4,
            (_, "汽车") | (_, "发送") | (_, "今天") => -5.0,
            (_, "送") | (_, "发") => -8.0,
            _ => return None,
        })
    }
}

const WORDS: &str = "邮箱\tyou xiang\t9000\n油箱\tyou xiang\t3000\n汽车\tqi che\t9000\n发送\tfa song\t9000\n发\tfa\t20000\n送\tsong\t20000\n今天\tjin tian\t9000\n";

fn context_engine() -> Engine {
    Engine::new(Dictionary::parse(WORDS).unwrap()).with_language_model(Box::new(ContextModel))
}

fn first(engine: &mut Engine, input: &str) -> String {
    engine.set_input(input);
    let text = engine.query().unwrap().candidates.items[0].text.clone();
    engine.clear();
    text
}

#[test]
fn left_context_takes_the_last_two_words_of_a_han_tail() {
    let model = ContextModel;
    assert_eq!(
        left_context_of("今天汽车", &model).context(),
        Context::after_two("今天", "汽车")
    );
    assert_eq!(left_context_of("汽车", &model).context(), Context::after("汽车"));
    // 标点、空格、字母结尾都是句首
    assert_eq!(left_context_of("汽车。", &model), LeftContext::default());
    assert_eq!(left_context_of("汽车 ", &model), LeftContext::default());
    assert_eq!(left_context_of("汽车abc", &model), LeftContext::default());
    assert_eq!(left_context_of("", &model), LeftContext::default());
    // 只看末尾 8 个字：前面再长也不影响
    let long = format!("{}今天汽车", "龘".repeat(20));
    assert_eq!(
        left_context_of(&long, &model).context(),
        Context::after_two("今天", "汽车")
    );
    // 模型一个词都不认识：句首
    assert_eq!(left_context_of("汽车", &NoLanguageModel), LeftContext::default());
}

#[test]
fn host_context_reorders_word_candidates_at_sentence_start() {
    let mut engine = context_engine();
    assert_eq!(first(&mut engine, "youxiang"), "邮箱");
    engine.history_mut().record("汽车");
    assert_eq!(first(&mut engine, "youxiang"), "油箱");
    // 壳给的前文压过本会话历史（`clear()` 会清掉壳给的前文，所以先喂拼音再给前文，与壳第一键的顺序一致）
    engine.set_input("youxiang");
    engine.set_rescoring_context(Some("请发送".to_owned()));
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "邮箱");
    engine.clear();
}

#[test]
fn punctuation_at_the_end_of_the_context_means_sentence_start() {
    let mut engine = context_engine();
    engine.history_mut().record("汽车，");
    assert_eq!(first(&mut engine, "youxiang"), "邮箱");
}

#[test]
fn private_input_ignores_the_context() {
    let mut engine = context_engine();
    engine.history_mut().record("汽车");
    engine.set_private(true);
    assert_eq!(first(&mut engine, "youxiang"), "邮箱");
    engine.set_private(false);
    engine.history_mut().record("汽车");
    assert_eq!(first(&mut engine, "youxiang"), "油箱");
}

#[test]
fn the_commit_chain_wins_over_host_context() {
    let mut engine = context_engine();
    engine.set_input("fasong");
    let candidate = engine.query().unwrap().candidates.items[0].clone();
    assert_eq!(candidate.text, "发送");
    engine.commit(&candidate);
    // 上屏把 发送 写进了历史；换成别的前文，链上仍是 发送
    engine.history_mut().clear();
    engine.history_mut().record("汽车");
    assert_eq!(first(&mut engine, "youxiang"), "邮箱");
    // 链断了就看前文
    engine.break_chain();
    engine.history_mut().clear();
    engine.history_mut().record("汽车");
    assert_eq!(first(&mut engine, "youxiang"), "油箱");
}

#[test]
fn the_first_word_of_a_sentence_sees_the_context() {
    let mut engine = context_engine();
    let sentence_of = |engine: &mut Engine| {
        engine.set_input("youxiangfasong");
        let text = engine
            .query()
            .unwrap()
            .candidates
            .items
            .iter()
            .find(|c| c.kind == CandidateKind::Sentence)
            .expect("有整句候选")
            .text
            .clone();
        engine.clear();
        text
    };
    assert_eq!(sentence_of(&mut engine), "邮箱发送");
    engine.history_mut().record("汽车");
    assert_eq!(sentence_of(&mut engine), "油箱发送");
}
```

`crates/qingjian-core/src/engine/tests/mod.rs` 的 `mod code;` 后加一行 `mod context_fork;`。

- [ ] **Step 2：跑，确认编译失败（`left_context` 不存在）**

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && cargo test -p qingjian-core context_fork 2>&1 | head -20
```

Expected: `error[E0432]: unresolved import`。

- [ ] **Step 3：写 `crates/qingjian-core/src/engine/query/left_context.rs`**

```rust
//! 词级排序与整句首词的上文：会话链上没有词（句首）时，从宿主光标前的文字末尾切出最后一两个词。
//!
//! 只看末尾 [`LEFT_CONTEXT_CHARS`] 个连续汉字，用静态语言模型最大匹配切词（[`sentence::segment_text`]）；
//! 末尾不是汉字（标点、空格、字母）就当句首，与上屏时标点打断链的规则一致。私密输入时不看前文。
//! 素笺分叉，见 cloud/docs/specs/2026-10-04-context-prediction-design.md。

use crate::engine::Engine;
use crate::sentence::{self, Context, LanguageModel, is_han};

/// 前文末尾最多看几个汉字。
pub(in crate::engine) const LEFT_CONTEXT_CHARS: usize = 8;

/// 拥有文本的上文，借出 [`Context`]。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(in crate::engine) struct LeftContext {
    /// 前一个词；`None` 是句首。
    previous: Option<String>,

    /// 再前一个词。
    earlier: Option<String>,
}

impl LeftContext {
    pub fn context(&self) -> Context<'_> {
        Context {
            previous: self.previous.as_deref(),
            earlier: self.earlier.as_deref(),
        }
    }
}

/// 从一段文字的末尾切出上文。
pub(in crate::engine) fn left_context_of(text: &str, model: &dyn LanguageModel) -> LeftContext {
    let mut tail: Vec<char> = text
        .chars()
        .rev()
        .take_while(|c| is_han(*c))
        .take(LEFT_CONTEXT_CHARS)
        .collect();
    if tail.is_empty() {
        return LeftContext::default();
    }
    tail.reverse();
    let tail: String = tail.into_iter().collect();
    let Some(mut clauses) = sentence::segment_text(&tail, model) else {
        return LeftContext::default();
    };
    let Some(mut words) = clauses.pop() else {
        return LeftContext::default();
    };
    let previous = words.pop();
    let earlier = words.pop();
    LeftContext { previous, earlier }
}

impl Engine {
    /// 下一个词的上文：链上有词就用链（本会话刚上屏的词最可信），否则从宿主前文末尾切
    /// （壳没给前文时 [`Self::rescoring_context`] 退回本会话历史）。私密输入时只认链，不看前文。
    pub(in crate::engine) fn word_context(&self) -> LeftContext {
        let chain = self.chain.context();
        if let Some(previous) = chain.previous {
            return LeftContext {
                previous: Some(previous.to_owned()),
                earlier: chain.earlier.map(str::to_owned),
            };
        }
        if self.is_private() {
            return LeftContext::default();
        }
        left_context_of(&self.rescoring_context(), &*self.language_model)
    }
}
```

`crates/qingjian-core/src/engine/query/mod.rs` 的 `mod english_tail;` 后加 `pub(in crate::engine) mod left_context;`。

- [ ] **Step 4：`phonetic.rs` 词级排序用它**

把 `phonetic.rs:145-164` 改成：

```rust
        let log_total = (self.total_frequency() as f64).max(1.0).ln();
        let letters = choice_key(scope, scope.len());
        // 上下文：链上的上一个词，链空着就是宿主前文末尾的词（素笺分叉，见 query/left_context.rs）
        let context = self.word_context();
        ranking::rank(&mut scored, MAX_CANDIDATES, |item| {
            let hit = &item.hit;
            // 纠错生效时覆盖的是纠正后的字母，换算回原串再查「这个输入串下选过什么」
            let covered = correction
                .as_ref()
                .map_or(item.coverage, |c| c.edit.to_original(item.coverage));
            let choice = letters
                .get(..covered)
                .map_or(0, |input| self.learner.choice_weight(input, hit.text));
            let log_prob = sentence::transition_log_prob(
                &*self.language_model,
                self.personal(),
                context.context(),
                hit.text,
                sentence::fallback_log_prob(hit.frequency, log_total),
            );
            (choice, log_prob)
        });
```

- [ ] **Step 5：`viterbi.rs` 首词带上文**

`convert_paths` 签名在 `k: usize,` 后加一个参数，函数体第一行把它改名（循环变量也叫 `start`）：

```rust
/// 得分最高的前 `k` 条路径（最多束宽条，按得分降序，文本相同的只留一条）：给重打分用。
/// `start` 是第一个词的上文（句首给 [`Context::START`]；素笺让宿主前文从这里进来）。
#[allow(clippy::too_many_arguments)]
pub fn convert_paths(
    dictionaries: &[&Dictionary],
    positions: &[Vec<SyllablePattern<'_>>],
    keep_partial: bool,
    k: usize,
    start: Context<'_>,
    model: &dyn LanguageModel,
    personal: Personal<'_>,
    weight: impl Fn(&str) -> u32,
    cost: impl Fn(usize, &str) -> f64,
    cache: &mut SpanCache,
) -> Vec<Conversion> {
    let first = start;
```

`convert_with` 里的调用在 `1,` 后补 `Context::START,`。

循环里两处（原 `best_predecessor(&nodes, start, &hit.text, model, personal, fallback)` 与 `static_step`）改成：

```rust
                let (score, back) =
                    best_predecessor(&nodes, start, &hit.text, first, model, personal, fallback);
                let previous = &nodes[start][back];
                let penalty = previous.penalty + hit.penalty;
                let static_previous = if start > 0 {
                    Some(previous.text.as_str())
                } else {
                    first.previous
                };
                let static_step = model
                    .log_prob(static_previous, &hit.text)
                    .unwrap_or(fallback);
```

占位音节那一处同样在 `text,` 后传 `first,`。`best_predecessor` 加参数并在 `start == 0` 时用它：

```rust
/// 在 `nodes[start]` 的前驱里挑让 `word` 得分最高的那条，返回 (累计得分, 前驱下标)。
/// 转移概率先问静态模型（不认识就用词库兜底值），再与个人 n-gram 插值；前二词是前驱自己的前驱（回指）。
/// 第一个词的上文是 `first`（句首或宿主前文末尾的词）。
#[allow(clippy::too_many_arguments)]
fn best_predecessor(
    nodes: &[Vec<Node>],
    start: usize,
    word: &str,
    first: Context<'_>,
    model: &dyn LanguageModel,
    personal: Personal<'_>,
    fallback: f64,
) -> (f64, usize) {
    let mut best = (f64::NEG_INFINITY, 0);
    for (index, previous) in nodes[start].iter().enumerate() {
        let context = if start == 0 {
            first
        } else {
            Context {
                previous: Some(previous.text.as_str()),
                earlier: (previous.start > 0)
                    .then(|| nodes[previous.start][previous.back].text.as_str()),
            }
        };
        let score = previous.score + transition_log_prob(model, personal, context, word, fallback);
        if score > best.0 {
            best = (score, index);
        }
    }
    best
}
```

`converting.rs` 的 `sentence_paths` 调用：

```rust
        let context = self.word_context();
        let mut paths = sentence::convert_paths(
            &dictionaries,
            &expanded.positions(),
            whole,
            k,
            context.context(),
            &*self.language_model,
            self.personal(),
            |text| self.learner.weight(text),
            |index, syllable| expanded.cost(index, syllable),
            &mut self.span_cache.borrow_mut(),
        );
```

格子缓存 `SpanCache` 只缓存每格的词，与上文无关，不用清。

- [ ] **Step 6：跑测试**

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && cargo test -p qingjian-core
```

Expected: 全绿，含 `context_fork` 6 个。上游 `viterbi` 的测试走 `convert` / `convert_whole`，不受影响。

- [ ] **Step 7：macOS 第一键总是读前文，私密不读**

`apps/macos/src/imk/controller/display.rs:11-25` 改成：

```rust
        // 前文现在也给词级排序与整句首词（素笺分叉，Core 的 query/left_context.rs），有没有模型都读；
        // 一段组句只在第一键读一次（组句中它不变；应用偶尔不回话也不至于让前文来回换）。
        // Secure Input 与引擎的私密输入都不读
        let wants_context = host::with(|h| {
            h.attach_loaded_model();
            h.engine.composition().text().chars().count() == 1 && !h.engine.is_private()
        })
        .unwrap_or(false);
        let before = if wants_context && !secure_input::enabled() {
            Some(
                client
                    .surrounding_text(RESCORE_LOOKBACK, 0)
                    .map(|text| text.before),
            )
        } else {
            None
        };
```

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && cargo clippy -p qingjian-macos --all-targets -- -D warnings
```

- [ ] **Step 8：评测（产品配置 = 对照 = 只有通变）**

跑「评测命令」里对照那条，填 Task 2 行。看「期望不在候选」与没命中例子，是评测集问题就改评测集（单独一个 `test(cli)` 提交）。门槛越线就停下报告。
**验收另加一条（审计定）**：`--eval-context-details` 里「我现在 → 又想」这类期望是整句（不是词库词）的对，有前文时要命中——前文进了 Viterbi 起点后
最优路径该变成「又 + 想」，整句候选按现有规则排到最前。把这些对在 details 里挑出来逐条看，没命中的列进报告。

- [ ] **Step 9：记 fork-patch、提交**

```markdown
| `crates/qingjian-core/src/engine/query/left_context.rs` | 新文件 | 前文末尾 8 字 → `Context`；`Engine::word_context`（链优先，私密不看） |
| `crates/qingjian-core/src/engine/query/mod.rs` | 加 1 行 | `mod left_context;` |
| `crates/qingjian-core/src/engine/query/phonetic.rs` | 改 2 行 | 词级排序的上下文改用 `word_context()` |
| `crates/qingjian-core/src/engine/query/converting.rs` | 加 2 行 | `convert_paths` 传首词上文 |
| `crates/qingjian-core/src/sentence/viterbi.rs` | 加 1 个参数 | `convert_paths` / `best_predecessor` 的 `start` / `first`：首词上文 |
| `crates/qingjian-core/src/engine/tests/context_fork.rs`、`tests/mod.rs` | 新文件、加 1 行 | 测试 |
| `apps/macos/src/imk/controller/display.rs` | 改 4 行 | 第一键总是读应用前文（私密不读），不再只在有模型时读 |
```

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && cargo fmt --all && git add -A crates/qingjian-core apps/macos/src/imk/controller/display.rs cloud/docs && git commit -m "feat(core): 宿主前文进词级排序与整句首词

- 链空着（句首）时用静态模型切前文末尾 8 字，取最后一两个词当 Context；私密输入不看
- Viterbi 首词也用它，不再固定 START；标点结尾视为句首
- macOS 第一键总是读应用前文；评测见 cloud/docs/plans/2026-10-04-context-prediction.md"
```

---

## Task 3：choice 改为加分

**Files:**
- Create: `crates/qingjian-core/src/ranking/choice_bonus.rs`
- Modify: `crates/qingjian-core/src/ranking/mod.rs`（`mod` + `rank` 闭包签名 + 测试）
- Modify: `crates/qingjian-core/src/ranking/scored.rs:37-48, 75-88`
- Modify: `crates/qingjian-core/src/engine/mod.rs`（字段）、`setup.rs`（setter）、`query/phonetic.rs`、`query/code.rs`
- Modify: `apps/cli/src/tuning.rs`（`choice` 键）
- Modify: `crates/qingjian-core/src/engine/tests/context_fork.rs`（三条测试）
- 可能 Modify: `crates/qingjian-core/src/engine/tests/learning.rs`（见 Step 6，改前先报审计）

现状：`SortKey` 第五项 `Reverse(choice)` 排在上下文得分前面，同一输入串下选过一次就永远第一。改成分数里加 `β · ln(1 + choice)`。

- [ ] **Step 1：先写 Engine 级测试，确认失败**

`tests/context_fork.rs` 顶部加 `use std::collections::HashMap;` 与 `use super::CountingLearner;`，末尾加：

```rust
fn learning_engine() -> Engine {
    context_engine().with_learner(Box::new(CountingLearner(HashMap::new())))
}

#[test]
fn a_single_past_choice_yields_to_strong_context() {
    let mut engine = learning_engine();
    engine.learner_mut().record_choice("youxiang", "邮箱");
    engine.history_mut().record("汽车");
    // 选过一次 邮箱，但 汽车 后面 油箱 领先 7 nat
    assert_eq!(first(&mut engine, "youxiang"), "油箱");
}

#[test]
fn a_single_past_choice_still_wins_under_weak_context() {
    let mut engine = learning_engine();
    engine.learner_mut().record_choice("youxiang", "油箱");
    // 句首 邮箱 只领先 0.4 nat，小于 β·ln2：选过一次的 油箱 第一
    assert_eq!(first(&mut engine, "youxiang"), "油箱");
}

#[test]
fn many_past_choices_still_beat_weak_context() {
    let mut engine = learning_engine();
    for _ in 0..5 {
        engine.learner_mut().record_choice("youxiang", "油箱");
    }
    assert_eq!(first(&mut engine, "youxiang"), "油箱");
}
```

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && cargo test -p qingjian-core context_fork
```

Expected: `a_single_past_choice_yields_to_strong_context` 失败（现状 choice 压过上下文，首选是 邮箱），另两条通过。

- [ ] **Step 2：写 `ranking/choice_bonus.rs`**

```rust
//! 「同一输入串下选过」的加分。原来是排序键里压在上下文得分之前的一级，选过一次「邮箱」上文怎么写都是它第一；
//! 改成对数加分，强上文翻得过只选过一两次的词，选过很多次的仍压得住弱上文。
//! β 按 `qingjian-cli --replay` 不降、`--eval-context` 最好取，见 cloud/docs/plans/2026-10-04-context-prediction.md。

/// 加分系数 β：加分 = β · ln(1 + 次数)。
pub const CHOICE_BONUS: f64 = 2.0;

/// 选过 `count` 次换成的得分加成。
pub fn choice_bonus(count: u32, weight: f64) -> f64 {
    weight * (1.0 + f64::from(count)).ln()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grows_with_the_log_of_the_count() {
        assert_eq!(choice_bonus(0, CHOICE_BONUS), 0.0);
        assert!(choice_bonus(1, CHOICE_BONUS) < choice_bonus(5, CHOICE_BONUS));
        assert!((choice_bonus(1, 1.0) - std::f64::consts::LN_2).abs() < 1e-12);
    }
}
```

- [ ] **Step 3：`ranking/mod.rs`：导出、`rank` 的闭包只返回分数**

`mod scored;` 后加 `mod choice_bonus;`，`pub use scored::…` 后加 `pub use choice_bonus::{CHOICE_BONUS, choice_bonus};`。
模块文档第 5 条改成：

```rust
//! 5. 上下文得分：语言模型给的 `log P(词 | 上一个词)`（个人 bigram 插值，模型不认识的按词库词频兜底并扣分，
//!    见 `sentence::transition_log_prob`）加用户选择次数的加分（[`weight_bonus`]，对数且封顶）、
//!    加同一输入串下选过的加分（[`choice_bonus`]：`mgs` 选过 美国式，下次 `mgs` 它多半还是首选，但强上文翻得过只选过一次的），
//!    模糊音命中扣 ln 2、敲错变体命中扣那类敲错的代价。这样 `ba` 在「做了」后面出 吧、句首出 把
//! 6. 敲的原音节优先，词长短者优先，最后按字符串稳定排序保证结果可复现
```

`rank`：

```rust
/// 排序并按词文本去重（同一个词可能被多种切分命中，保留得分最高的一条），最多留 `limit` 条。
/// `context` 给每条命中算上下文 log 概率（已含同输入串下选过的加分），只对预选后剩下的那些调用。
pub fn rank(items: &mut Vec<Scored<'_>>, limit: usize, context: impl Fn(&Scored<'_>) -> f64) {
    let preselect = limit.saturating_mul(2);
    if items.len() > preselect.saturating_mul(2) {
        let mut keyed: Vec<(PreselectKey, Scored<'_>)> = items
            .drain(..)
            .map(|item| (item.preselect_key(), item))
            .collect();
        keyed.select_nth_unstable_by(preselect, |a, b| b.0.cmp(&a.0));
        keyed.truncate(preselect);
        items.extend(keyed.into_iter().map(|(_, item)| item));
    }
    let mut keyed: Vec<(SortKey<'_>, Scored<'_>)> = items
        .drain(..)
        .map(|item| {
            let score = context(&item) + weight_bonus(item.weight) - item.penalty;
            (item.key(score), item)
        })
        .collect();
    keyed.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    let mut seen: HashSet<&str> = HashSet::with_capacity(limit.min(keyed.len()));
    items.extend(
        keyed
            .into_iter()
            .map(|(_, item)| item)
            .filter(|item| seen.insert(item.hit.text))
            .take(limit),
    );
}
```

`scored.rs`：`SortKey` 去掉 `Reverse<u32>` 那一项，`key` 只收 `score`：

```rust
/// 排序键，越小越靠前。元组的顺序即排序规则，见模块文档；第五项是上下文得分（毫分，整数才能比较，
/// 已含选择次数与同输入串选择的加分）。文本借自词库，键可以脱离 `Scored` 存放。
pub type SortKey<'a> = (
    Reverse<bool>,
    Reverse<usize>,
    usize,
    Reverse<bool>,
    Reverse<i64>,
    bool,
    usize,
    &'a str,
);
```

```rust
    /// `score` 是上下文得分（log 概率，已含用户加分、同输入串选择加分与模糊音 / 敲错扣分）。
    pub(super) fn key(&self, score: f64) -> SortKey<'a> {
        (
            Reverse(self.hit.exact),
            Reverse(self.coverage),
            self.abbreviated,
            Reverse(self.full_last),
            Reverse((score * 1000.0).round() as i64),
            self.altered(),
            self.hit.text.chars().count(),
            self.hit.text,
        )
    }
```

`ranking/mod.rs` 的测试：所有 `rank(&mut items, usize::MAX, |_| (0, 0.0))` 改成 `|_| 0.0`、`|_| (0, -2.0)` 改成 `|_| -2.0`；
`context_score_orders_within_the_same_structure` 里 `by_context` 改成返回 `f64`，中间「同一输入串下选过的压过上下文」那一段换成：

```rust
        // 同一输入串下选过一次：加分翻不过 5 nat 的强上文
        rank(&mut items, usize::MAX, |s| {
            let choice = u32::from(s.hit.text == "把");
            (if s.hit.text == "吧" { -1.0 } else { -6.0 }) + choice_bonus(choice, CHOICE_BONUS)
        });
        assert_eq!(items[0].hit.text, "吧");
        // 选过一次：翻得过 0.5 nat 的弱上文（β·ln2 ≥ 0.5 对所有候选 β 成立）
        rank(&mut items, usize::MAX, |s| {
            let choice = u32::from(s.hit.text == "把");
            (if s.hit.text == "吧" { -1.0 } else { -1.5 }) + choice_bonus(choice, CHOICE_BONUS)
        });
        assert_eq!(items[0].hit.text, "把");
```

- [ ] **Step 4：Engine 字段、setter、两处调用、`--tune choice`**

`engine/mod.rs` 在 `neural_context: usize,` 字段后加：

```rust
    /// 同一输入串下选过的加分系数 β（[`crate::ranking::CHOICE_BONUS`]）；只有回放调参会改。
    choice_bonus: f64,
```

`Engine::new` 里 `neural_context: RESCORE_CONTEXT_CHARS,` 后加 `choice_bonus: crate::ranking::CHOICE_BONUS,`。

`setup.rs` 在 `pub fn typo_costs` 后加：

```rust
    /// 换同输入串选择加分的系数 β（回放调参用）。
    pub fn set_choice_bonus(&mut self, weight: f64) {
        self.choice_bonus = weight.max(0.0);
    }

    pub fn choice_bonus(&self) -> f64 {
        self.choice_bonus
    }
```

`phonetic.rs` 的闭包结尾 `(choice, log_prob)` 改成 `log_prob + ranking::choice_bonus(choice, self.choice_bonus)`；
`code.rs:45-58` 同样把 `(choice, log_prob)` 改成 `log_prob + ranking::choice_bonus(choice, self.choice_bonus)`，
两处注释里「同一输入串下选过的优先」改成「同一输入串下选过的加分」。

`apps/cli/src/tuning.rs`：`KEYS` 改成 `[&str; 12]` 加 `"choice"`；`apply` 里：

```rust
    let mut choice: Option<f64> = None;
    …
            "choice" => choice = Some(value),
    …
    engine.set_interpolation(interpolation);
    engine.set_typo_costs(costs);
    if let Some(choice) = choice {
        engine.set_choice_bonus(choice);
    }
```

文档注释加 `choice（同输入串下选过的加分系数 β）`。

- [ ] **Step 5：跑测试**

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && cargo test -p qingjian-core && cargo test -p qingjian-cli
```

Expected: 全绿。

- [ ] **Step 6：上游 `engine/tests/learning.rs` 变红的处理**

有用例因「选过就第一」变红时：**先不改**。把每条红掉的用例名、它断言的分差、本任务的规则会怎么排，整理成一段发审计会话「素笺输入法」，
得到确认后再改断言；改了就在提交正文逐条写明，并在 fork-patch 表加一行 `crates/qingjian-core/src/engine/tests/learning.rs`。

- [ ] **Step 7：扫 β（`--replay` 必须不降）**

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && for b in 0.5 1 2 4; do echo "== choice=$b"; cargo run --release -q -p qingjian-cli -- --config /tmp/eval-config.toml --neural data/models/hanzhang-tongbian --neural-async --tune choice=$b --replay data/eval/input-log-2026-10-04.jsonl --misses 0 | grep '^词'; cargo run --release -q -p qingjian-cli -- --config /tmp/eval-config.toml --neural data/models/hanzhang-tongbian --neural-async --tune choice=$b --eval-context cloud/data/eval/context-pairs.tsv | grep 对数; done
```

只在 `--replay` 词首选 **≥ 基线**的 β 里挑 `--eval-context` 最好的，写进 `CHOICE_BONUS`；一个都不满足就停下，把四组数字发审计。

| β | `--replay` 词首选 | `--eval-context` 有前文 |
|---|---|---|
| 0.5 | | |
| 1 | | |
| 2 | | |
| 4 | | |

- [ ] **Step 8：评测（对照配置）、fork-patch、提交**

```markdown
| `crates/qingjian-core/src/ranking/choice_bonus.rs` | 新文件 | β 与 `choice_bonus` |
| `crates/qingjian-core/src/ranking/mod.rs`、`scored.rs` | 改签名 | `SortKey` 去掉 choice 那一级，`rank` 闭包只返回分数 |
| `crates/qingjian-core/src/engine/mod.rs`、`setup.rs` | 加字段与 setter | `choice_bonus`、`set_choice_bonus` |
| `crates/qingjian-core/src/engine/query/phonetic.rs`、`code.rs` | 各改 1 行 | 加分进分数 |
| `apps/cli/src/tuning.rs` | 加 1 个键 | `--tune choice=β` |
```

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && cargo fmt --all && git add -A crates/qingjian-core apps/cli cloud/docs && git commit -m "feat(core): 同输入串下选过的词改为加分，不再压过上下文

- 排序键去掉 choice 那一级，分数里加 β·ln(1+次数)
- β 以 --replay 不降为准、--eval-context 最好取（数字见计划文件），--tune choice= 可改"
```

---

## Task 4：知微给第一页词级候选按前文打分

**Files:**
- Create: `crates/qingjian-core/src/engine/rescoring/word_rescore/mod.rs`
- Create: `crates/qingjian-core/src/engine/rescoring/word_rescore/tests.rs`
- Modify: `crates/qingjian-core/src/engine/rescoring/mod.rs`（`mod` 1 行 + 导出 + `rescoring_pending` / `request_rescoring` / `poll_rescoring` 各加几行）
- Modify: `crates/qingjian-core/src/engine/mod.rs`（4 个字段）
- Modify: `crates/qingjian-core/src/ranking/mod.rs`（`rank` 返回分数）
- Modify: `crates/qingjian-core/src/engine/query/phonetic.rs`（2 行挂钩）、`query/code.rs`（`let _ =`）
- Modify: `apps/cli/src/args.rs`、`main.rs`（`--word-model`、`--word-weight`）

设计（审计定）：**只排词，不排整句。** 第一页里与首选**同一结构档**（`exact` / `coverage` / `abbreviated` / `full_last` 四项相同）的前 6 个
词级候选，每条问知微 `log P(候选 | 前文)`，按 `静态分 + λ_w·(神经分 − 静态分)` 在它们原来占的那几格之间换位；
整句候选、英文、快捷、emoji、别档的前缀词全都不动（整句路径分是按词累加的，与单个词的分不在一个尺度）。
静态分就是排序用的上下文得分（含用户加分、选择加分、扣分）。缓存与后台线程照搬整句重排那一套：
同步打分器（CLI 评测）当场补分；异步的先 `want`，壳停顿后 `request_rescoring` 一起送，`poll_rescoring` 到了再查一次。

- [ ] **Step 1：`ranking::rank` 返回每条的最终分**

```rust
/// 排序并按词文本去重（同一个词可能被多种切分命中，保留得分最高的一条），最多留 `limit` 条。
/// `context` 给每条命中算上下文 log 概率（已含同输入串下选过的加分），只对预选后剩下的那些调用。
/// 返回留下的每条的最终得分（上下文得分 + 用户加分 − 扣分），与 `items` 同序，给词级重排当静态分。
pub fn rank(
    items: &mut Vec<Scored<'_>>,
    limit: usize,
    context: impl Fn(&Scored<'_>) -> f64,
) -> Vec<f64> {
    …（预选不变）
    let mut keyed: Vec<(SortKey<'_>, f64, Scored<'_>)> = items
        .drain(..)
        .map(|item| {
            let score = context(&item) + weight_bonus(item.weight) - item.penalty;
            (item.key(score), score, item)
        })
        .collect();
    keyed.sort_unstable_by(|a, b| a.0.cmp(&b.0));
    let mut seen: HashSet<&str> = HashSet::with_capacity(limit.min(keyed.len()));
    let mut scores = Vec::with_capacity(limit.min(keyed.len()));
    items.extend(
        keyed
            .into_iter()
            .filter(|(_, _, item)| seen.insert(item.hit.text))
            .take(limit)
            .map(|(_, score, item)| {
                scores.push(score);
                item
            }),
    );
    scores
}
```

`code.rs` 的调用前加 `let _ =`（形码不重排）。`ranking/mod.rs` 测试里的 `rank(...)` 调用照旧。

- [ ] **Step 2：写失败的测试 `rescoring/word_rescore/tests.rs`**

```rust
//! 知微词级重排的测试：假打分器，不依赖模型文件。

use std::time::{Duration, Instant};

use qingjian_dictionary::Dictionary;

use crate::candidate::CandidateKind;
use crate::engine::Engine;
use crate::sentence::{LanguageModel, SentenceScorer};

/// 假知微：偏爱某个文本。
struct Prefers(&'static str);

impl SentenceScorer for Prefers {
    fn score(&self, _context: &str, _keys: &str, texts: &[&str]) -> Vec<f64> {
        texts
            .iter()
            .map(|t| if *t == self.0 { -1.0 } else { -20.0 })
            .collect()
    }
}

/// 假知微：算不了。
struct Broken;

impl SentenceScorer for Broken {
    fn score(&self, _context: &str, _keys: &str, _texts: &[&str]) -> Vec<f64> {
        Vec::new()
    }
}

/// 句首强烈偏向 又 + 想：`youxiang` 的整句首选是 又想。
struct LikesYouXiang;

impl LanguageModel for LikesYouXiang {
    fn log_prob(&self, previous: Option<&str>, word: &str) -> Option<f64> {
        Some(match (previous, word) {
            (None, "又") => -1.0,
            (Some("又"), "想") => -1.0,
            (_, "邮箱") | (_, "油箱") => -8.0,
            (_, "邮") | (_, "又") | (_, "想") => -6.0,
            _ => return None,
        })
    }
}

const WORDS: &str = "邮箱\tyou xiang\t9000\n油箱\tyou xiang\t3000\n邮\tyou\t900\n又\tyou\t800\n想\txiang\t900\n开发\tkai fa\t9000\n开\tkai\t20000\n";

fn engine() -> Engine {
    Engine::new(Dictionary::parse(WORDS).unwrap())
}

fn all(engine: &Engine) -> Vec<(CandidateKind, String)> {
    engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .map(|c| (c.kind, c.text))
        .collect()
}

fn words(engine: &Engine) -> Vec<String> {
    all(engine)
        .into_iter()
        .filter(|(kind, _)| *kind == CandidateKind::Chinese)
        .map(|(_, text)| text)
        .collect()
}

#[test]
fn sync_word_scorer_reorders_the_top_tier() {
    let mut engine = engine().with_word_scorer(Box::new(Prefers("油箱")), Some(1.0));
    engine.set_input("youxiang");
    assert_eq!(words(&engine)[..2], ["油箱", "邮箱"]);
    assert!(!engine.rescoring_pending());
}

#[test]
fn only_candidates_in_the_first_tier_move() {
    // 开 是前缀词（coverage 小），不与 开发 同档：打分器再偏爱它也翻不上来
    let mut engine = engine().with_word_scorer(Box::new(Prefers("开")), Some(1.0));
    engine.set_input("kaifa");
    assert_eq!(words(&engine)[0], "开发");
}

#[test]
fn the_sentence_candidate_is_never_moved() {
    // 「我现在 + youxiang」：整句 又想 排第一；打分器偏爱前缀词 邮，整句与 邮箱 都不该被压到 邮 下面
    let mut engine = engine()
        .with_language_model(Box::new(LikesYouXiang))
        .with_word_scorer(Box::new(Prefers("邮")), Some(1.0));
    engine.set_input("youxiang");
    let items = all(&engine);
    assert_eq!(items[0], (CandidateKind::Sentence, "又想".to_owned()));
    let position = |text: &str| items.iter().position(|(_, t)| t == text).unwrap();
    assert!(position("邮箱") < position("邮"));
}

#[test]
fn a_failing_scorer_keeps_the_static_order() {
    let mut engine = engine().with_word_scorer(Box::new(Broken), Some(1.0));
    engine.set_input("youxiang");
    assert_eq!(words(&engine)[..2], ["邮箱", "油箱"]);
}

#[test]
fn async_word_scorer_waits_for_request_and_poll() {
    let mut engine = engine().with_async_word_scorer(Box::new(Prefers("油箱")), Some(1.0));
    engine.set_input("youxiang");
    assert_eq!(words(&engine)[..2], ["邮箱", "油箱"]);
    assert!(engine.rescoring_pending());
    assert!(engine.request_rescoring());
    let started = Instant::now();
    while !engine.poll_rescoring() {
        assert!(started.elapsed() < Duration::from_secs(5), "后台没回结果");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(words(&engine)[..2], ["油箱", "邮箱"]);
    assert!(!engine.rescoring_pending());
}

#[test]
fn without_a_word_scorer_nothing_changes() {
    let mut engine = engine();
    engine.set_input("youxiang");
    assert_eq!(words(&engine)[..2], ["邮箱", "油箱"]);
    assert!(!engine.has_word_scorer());
}
```

`rescoring/mod.rs` 的 `mod worker;` 后加 `mod word_rescore;`。先建 `word_rescore/mod.rs` 只含文件头与 `#[cfg(test)] mod tests;`：

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && cargo test -p qingjian-core word_rescore 2>&1 | head
```

Expected: 编译错误（`with_word_scorer` 不存在）。

- [ ] **Step 3：Engine 字段**

`engine/mod.rs` 在 `neural_context: usize,` 后加：

```rust
    /// 知微：给第一页词级候选按前文打分的同步打分器（CLI 评测用）。素笺分叉，见 rescoring/word_rescore。
    word_scorer: Option<Box<dyn SentenceScorer>>,

    /// 异步的知微（壳里用）；与整句重排的线程分开，模型不同。
    word_rescorer: Option<rescoring::RescoreWorker>,

    /// 「前文 + 候选 → 知微分」缓存。
    word_cache: std::cell::RefCell<rescoring::NeuralCache>,

    /// 词级重排里神经分的权重 λ_w。
    word_weight: f64,
```

`Engine::new` 加：

```rust
            word_scorer: None,
            word_rescorer: None,
            word_cache: std::cell::RefCell::new(rescoring::NeuralCache::default()),
            word_weight: rescoring::WORD_NEURAL_WEIGHT,
```

`rescoring/mod.rs` 的 `pub(crate) use worker::RescoreWorker;` 后加 `pub use word_rescore::{WORD_NEURAL_WEIGHT, WORD_RESCORE_CANDIDATES};`。

- [ ] **Step 4：写 `rescoring/word_rescore/mod.rs`**

```rust
//! 知微给词级候选按前文打分：第一页里与首选同一结构档（精确 / 覆盖 / 简拼数 / 末音节完整都相同）的前几个词，
//! 按「静态分 + λ_w·(神经分 − 静态分)」重排，只在这几格之间换位；整句（路径分按词累加，与词不在一个尺度）、
//! 英文 / 快捷 / emoji 与别档的词都不动。打分走与整句重排同样的「缓存 + 后台线程 + 停顿后请求 + 结果到了再查一次」
//! （见 ../mod.rs），按键回调不等模型。素笺分叉，见 cloud/docs/specs/2026-10-04-context-prediction-design.md。

#[cfg(test)]
mod tests;

use std::collections::HashMap;

use crate::candidate::{Candidate, CandidateKind};
use crate::engine::Engine;
use crate::engine::rescoring::{NeuralCache, RescoreWorker};
use crate::ranking::Scored;
use crate::sentence::SentenceScorer;

/// 词级重排里神经分的缺省权重 λ_w，按 `--eval-context` 扫 {0.3, 0.5, 0.8} 取（见计划文件）。
pub const WORD_NEURAL_WEIGHT: f64 = 0.5;

/// 最多给几个词级候选打分。
pub const WORD_RESCORE_CANDIDATES: usize = 6;

impl Engine {
    /// 挂上同步的知微（查询里当场打，评测用）。`weight` 是 λ_w，`None` 用 [`WORD_NEURAL_WEIGHT`]。
    pub fn with_word_scorer(mut self, scorer: Box<dyn SentenceScorer>, weight: Option<f64>) -> Self {
        self.word_scorer = Some(scorer);
        self.word_rescorer = None;
        self.word_weight = weight.unwrap_or(WORD_NEURAL_WEIGHT).clamp(0.0, 1.0);
        self
    }

    /// 挂上异步的知微（后台线程，壳里用）。
    pub fn with_async_word_scorer(
        mut self,
        scorer: Box<dyn SentenceScorer>,
        weight: Option<f64>,
    ) -> Self {
        self.set_async_word_scorer(Some(scorer));
        self.word_weight = weight.unwrap_or(WORD_NEURAL_WEIGHT).clamp(0.0, 1.0);
        self
    }

    /// 运行时换 / 卸异步知微（壳里模型后台加载完才接上，配置关掉就卸）。
    pub fn set_async_word_scorer(&mut self, scorer: Option<Box<dyn SentenceScorer>>) {
        self.word_scorer = None;
        self.word_rescorer = scorer.map(RescoreWorker::spawn);
        *self.word_cache.borrow_mut() = NeuralCache::default();
    }

    pub fn has_word_scorer(&self) -> bool {
        self.word_scorer.is_some() || self.word_rescorer.as_ref().is_some_and(RescoreWorker::is_alive)
    }

    pub fn set_word_weight(&mut self, weight: f64) {
        self.word_weight = weight.clamp(0.0, 1.0);
    }

    /// 排好序的词级命中里要给知微打分的那一档：与第一条同结构档的前几条，带静态分。没接知微返回空。
    pub(in crate::engine) fn word_tier(&self, scored: &[Scored<'_>], scores: &[f64]) -> Vec<(String, f64)> {
        let (true, Some(head)) = (self.has_word_scorer(), scored.first()) else {
            return Vec::new();
        };
        let same = |s: &Scored<'_>| {
            (s.hit.exact, s.coverage, s.abbreviated, s.full_last)
                == (head.hit.exact, head.coverage, head.abbreviated, head.full_last)
        };
        scored
            .iter()
            .zip(scores)
            .take(WORD_RESCORE_CANDIDATES)
            .take_while(|(s, _)| same(s))
            .map(|(s, score)| (s.hit.text.to_owned(), *score))
            .collect()
    }

    /// 把第一页里的同档词按知微重排。异步时缺分就记下等壳来取、这次不动；任何一条没分也不动（半截重排比不重排还糟）。
    pub(in crate::engine) fn rescore_first_page(&self, items: &mut [Candidate], tier: &[(String, f64)]) {
        if !self.has_word_scorer() || tier.len() < 2 {
            return;
        }
        let statics: HashMap<&str, f64> = tier.iter().map(|(t, s)| (t.as_str(), *s)).collect();
        let slots: Vec<usize> = items
            .iter()
            .enumerate()
            .filter(|(_, c)| c.kind == CandidateKind::Chinese && statics.contains_key(c.text.as_str()))
            .map(|(index, _)| index)
            .collect();
        if slots.len() < 2 {
            return;
        }
        let context = self.rescoring_context();
        let keys = self.composition.scope().to_owned();
        let mut cache = self.word_cache.borrow_mut();
        cache.ensure_condition(&context, &keys);
        let missing: Vec<String> = slots
            .iter()
            .map(|&i| items[i].text.clone())
            .filter(|text| cache.get(text).is_none())
            .collect();
        if !missing.is_empty() {
            match &self.word_scorer {
                Some(scorer) => {
                    let texts: Vec<&str> = missing.iter().map(String::as_str).collect();
                    let scores = scorer.score(&context, &keys, &texts);
                    if scores.len() != texts.len() {
                        return;
                    }
                    for (text, score) in texts.iter().zip(scores) {
                        cache.insert(text, score);
                    }
                }
                None => {
                    for text in &missing {
                        cache.want(text);
                    }
                    return;
                }
            }
        }
        let lambda = self.word_weight;
        let rescored = |index: usize| {
            let text = items[index].text.as_str();
            let static_score = statics[text];
            static_score + lambda * (cache.get(text).expect("filled above") - static_score)
        };
        let mut order = slots.clone();
        order.sort_by(|&a, &b| {
            rescored(b)
                .partial_cmp(&rescored(a))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        if order == slots {
            return;
        }
        let moved: Vec<Candidate> = order.iter().map(|&i| items[i].clone()).collect();
        for (slot, candidate) in slots.into_iter().zip(moved) {
            items[slot] = candidate;
        }
        self.last_rescored.set(true);
    }

    /// 知微那边有没分的候选等着送后台。
    pub(super) fn word_rescoring_pending(&self) -> bool {
        self.word_rescorer.is_some() && self.word_cache.borrow().has_wanted()
    }

    /// 把攒着的候选送去知微的后台线程；没什么要送返回 `false`。
    pub(super) fn request_word_rescoring(&mut self) -> bool {
        let Some(worker) = &self.word_rescorer else {
            return false;
        };
        let mut cache = self.word_cache.borrow_mut();
        let wanted = cache.take_wanted();
        if wanted.is_empty() {
            return false;
        }
        tracing::debug!(texts = wanted.len(), "知微词级请求");
        worker.submit(cache.context().to_owned(), cache.keys().to_owned(), wanted, None);
        true
    }

    /// 收知微打好的分；有新分进了缓存返回 `true`。前文或按键已经变了的结果丢掉。
    pub(super) fn poll_word_rescoring(&mut self) -> bool {
        let Some(worker) = &self.word_rescorer else {
            return false;
        };
        let mut updated = false;
        while let Some(scored) = worker.poll() {
            let mut cache = self.word_cache.borrow_mut();
            if scored.context != cache.context()
                || scored.keys != cache.keys()
                || scored.scores.len() != scored.texts.len()
            {
                continue;
            }
            for (text, score) in scored.texts.iter().zip(scored.scores) {
                cache.insert(text, score);
            }
            updated = true;
        }
        updated
    }
}
```

（`Scored` 要从 `crate::ranking` 公开：它已经 `pub use scored::Scored`。）

- [ ] **Step 5：`rescoring/mod.rs` 三个入口带上知微**

`rescoring_pending`：

```rust
    pub fn rescoring_pending(&self) -> bool {
        self.word_rescoring_pending()
            || self.rescorer.is_some() && {
                let cache = self.neural_cache.borrow();
                cache.has_wanted() || cache.wanted_generation().is_some()
            }
    }
```

`request_rescoring` 开头与两处 `return false`：

```rust
    pub fn request_rescoring(&mut self) -> bool {
        let sent = self.request_word_rescoring();
        let Some(worker) = &self.rescorer else {
            return sent;
        };
        let mut cache = self.neural_cache.borrow_mut();
        let wanted = cache.take_wanted();
        let generate = cache.take_wanted_generation();
        if wanted.is_empty() && generate.is_none() {
            return sent;
        }
```

`poll_rescoring` 开头：

```rust
    pub fn poll_rescoring(&mut self) -> bool {
        let mut updated = self.poll_word_rescoring();
        let Some(worker) = &self.rescorer else {
            return updated;
        };
```

（原来的 `let mut updated = false;` 删掉。）

- [ ] **Step 6：`phonetic.rs` 两行挂钩**

`ranking::rank(...)` 改成接返回值并取档：

```rust
        let scores = ranking::rank(&mut scored, MAX_CANDIDATES, |item| { … });
        // 知微重排的对象（素笺分叉，见 rescoring/word_rescore）
        let tier = self.word_tier(&scored, &scores);
```

在 `let rank = start.elapsed();` 之前（`if aux_code.is_none() { … }` 的大括号之后）加：

```rust
        self.rescore_first_page(&mut items, &tier);
```

- [ ] **Step 7：跑测试**

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && cargo test -p qingjian-core
```

Expected: 全绿，含 `word_rescore` 6 个。

- [ ] **Step 8：CLI 接知微**

`args.rs` 在 `neural_async` 后加：

```rust
    /// 知微（含章·知微，字级模型）的 .qjm 或导出目录：给第一页词级候选按前文打分重排（素笺）
    #[arg(long)]
    pub word_model: Option<PathBuf>,

    /// 词级重排里神经分的权重 λ_w（0 到 1，缺省 0.5）
    #[arg(long, requires = "word_model")]
    pub word_weight: Option<f64>,
```

`main.rs` 的 `if let Some(path) = &args.eval_p2c {` 之前加：

```rust
    if let Some(path) = &args.word_model {
        let started = Instant::now();
        let scorer = qingjian_neural::CharScorer::load(path)?;
        if scorer.vocab().sep().is_some() {
            return Err(qingjian_neural::NeuralError::Corrupt(
                "--word-model expects Hanzhang Zhiwei (a character LM without <sep>)",
            )
            .into());
        }
        tracing::info!(load_ms = started.elapsed().as_millis(), "知微词级重排已启用");
        engine = if args.neural_async {
            engine.with_async_word_scorer(Box::new(scorer), args.word_weight)
        } else {
            engine.with_word_scorer(Box::new(scorer), args.word_weight)
        };
    }
```

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && cargo run --release -p qingjian-cli -- --word-model data/models/hanzhang-zhiwei youxiang
```

Expected: 打出候选，日志有「知微词级重排已启用」。

- [ ] **Step 9：扫 λ_w**

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && for w in 0.3 0.5 0.8; do echo "== word-weight=$w"; cargo run --release -p qingjian-cli -- --neural data/models/hanzhang-tongbian --neural-async --word-model data/models/hanzhang-zhiwei --word-weight $w --eval-context cloud/data/eval/context-pairs.tsv | grep 对数; done
```

取最好的写进 `WORD_NEURAL_WEIGHT`：

| λ_w | `--eval-context` 有前文 / 无前文 |
|---|---|
| 0.3 | |
| 0.5 | |
| 0.8 | |

- [ ] **Step 10：评测（产品配置从这里起带 `--word-model`；对照一起跑）、按键 p99、fork-patch、提交**

两条评测命令都跑，填 Task 4 行；`--replay` 与 `--eval-context` 报的同步 p99 比基线不超过 +1 ms（异步接法下查询不等模型，差的只是取档与查缓存）。
超了就停下报告。

```markdown
| `crates/qingjian-core/src/engine/rescoring/word_rescore/mod.rs`、`tests.rs` | 新文件 | 知微词级重排、取档、请求与收结果；测试 |
| `crates/qingjian-core/src/engine/rescoring/mod.rs` | 加 1 行 mod、1 行导出，改 3 个入口各几行 | `rescoring_pending` / `request_rescoring` / `poll_rescoring` 带上知微 |
| `crates/qingjian-core/src/engine/mod.rs` | 加 4 个字段 | `word_scorer` / `word_rescorer` / `word_cache` / `word_weight` |
| `crates/qingjian-core/src/ranking/mod.rs` | 改返回值 | `rank` 返回每条的最终分 |
| `crates/qingjian-core/src/engine/query/phonetic.rs` | 加 2 行 | 取档、调 `rescore_first_page` |
| `crates/qingjian-core/src/engine/query/code.rs` | 改 1 行 | `let _ = ranking::rank(…)` |
| `apps/cli/src/args.rs`、`main.rs` | 加 2 个参数与加载 | `--word-model`、`--word-weight` |
```

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && cargo fmt --all && git add -A crates/qingjian-core apps/cli cloud/docs && git commit -m "feat(core): 知微给第一页词级候选按前文打分重排

- 与首选同档的前 6 个词按 静态分 + λ_w·(神经分 − 静态分) 换位；整句与别档的词不动
- 走与整句重排同样的缓存 / 后台线程 / 停顿后请求，按键不等模型
- CLI --word-model / --word-weight；λ_w 扫出见计划文件"
```

---

## Task 5：`[model] scorers = "both"`，macOS 同时加载通变与知微

**Files:**
- Create: `crates/qingjian-platform/src/config/scorer_set.rs`
- Modify: `crates/qingjian-platform/src/config/model.rs`、`mod.rs`（`mod` / `pub use` / 模板 2 行 / 1 条测试）
- Create: `apps/macos/src/host/model/word_model.rs`
- Modify: `apps/macos/src/host/model/mod.rs`（挂钩 4 处）、`apps/macos/src/host/mod.rs`（1 个字段）、`host/init.rs`（初值）
- Modify: `cloud/docs/design.md`（一小节）、`fork-patch.md`

- [ ] **Step 1：配置项**

`crates/qingjian-platform/src/config/scorer_set.rs`：

```rust
//! `[model] scorers`：加载哪几个本地模型（素笺分叉）。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ScorerSet {
    /// 只有含章·通变（上游行为）：按键 → 整句路径重排与生成。
    Tongbian,

    /// 通变照旧，再加含章·知微按前文给词级候选打分、做本地续写（素笺缺省；多约 56 MB 常驻内存）。
    #[default]
    Both,
}
```

`model.rs`：

```rust
use serde::{Deserialize, Serialize};

use super::ScorerSet;

/// 配置文件 `[model]` 分节：本地模型的开关。
///
/// 随包的字级小模型在本机给整句候选重新排序，全程离线、不联网，与云联想互不影响（本地先出、云端到了另占它自己的格）。
/// 模型文件不在包里（或用户目录 `models/` 里）时开关无效。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LocalModelConfig {
    /// 开着就加载模型、给整句重排。
    pub enabled: bool,

    /// 加载哪几个模型（素笺分叉）。
    pub scorers: ScorerSet,
}

impl Default for LocalModelConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            scorers: ScorerSet::default(),
        }
    }
}
```

`config/mod.rs`：`mod model;` 旁加 `mod scorer_set;`，`pub use model::LocalModelConfig;` 旁加 `pub use scorer_set::ScorerSet;`；
模板 `[model]` 段 `enabled = true` 之后加：

```toml
# 加载哪几个模型：both = 通变（整句）+ 知微（按前文排词、本地续写）；tongbian = 只有通变
scorers = "both"
```

测试模块里加：

```rust
    #[test]
    fn model_scorers_parse_and_default_to_both() {
        let config: Config = toml::from_str("[model]\nscorers = \"tongbian\"\n").unwrap();
        assert_eq!(config.model.scorers, ScorerSet::Tongbian);
        assert!(config.model.enabled);
        assert_eq!(Config::default().model.scorers, ScorerSet::Both);
    }
```

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && cargo test -p qingjian-platform
```

Expected: 全绿（`template_parses_to_defaults` 也过）。

- [ ] **Step 2：macOS 加载知微；切到 `tongbian` 真卸掉**

`apps/macos/src/host/mod.rs` 在 `model_loader` 字段后加：

```rust
    /// 正在后台加载的知微（素笺：按前文排词与续写）；接上就清掉。
    word_loader: Option<
        std::sync::mpsc::Receiver<
            Result<Box<dyn qingjian_core::sentence::SentenceScorer>, qingjian_neural::NeuralError>,
        >,
    >,
```

`host/init.rs` 构造 Host 的地方（`model_loader: None,` 旁）加 `word_loader: None,`。

新文件 `apps/macos/src/host/model/word_model.rs`：

```rust
//! 知微（含章·知微）在壳里的加载：`[model] scorers = "both"` 时与通变一起在后台加载，接到 Engine 的词级重排上；
//! 配置切回 `tongbian` 时真卸掉（内存与日志对比要准）。素笺分叉，见 cloud/docs/fork-patch.md。

use std::sync::mpsc::{TryRecvError, channel};

use qingjian_core::sentence::SentenceScorer;
use qingjian_neural::{CharScorer, NeuralError};
use qingjian_platform::ScorerSet;

use crate::app::paths;
use crate::host::Host;

impl Host {
    /// 后台加载知微并预热。配置不要就卸掉已有的；没有文件或已经在加载 / 已接上就什么都不做。
    pub(super) fn load_word_model(&mut self) {
        if self.settings.config().model.scorers != ScorerSet::Both {
            self.unload_word_model();
            return;
        }
        if self.word_loader.is_some() || self.engine.has_word_scorer() {
            return;
        }
        let Some(path) = paths::model_path() else {
            tracing::info!("没有知微模型文件，不按前文排词");
            return;
        };
        let (tx, rx) = channel::<Result<Box<dyn SentenceScorer>, NeuralError>>();
        let spawned = std::thread::Builder::new()
            .name("qingjian-word-model-load".to_owned())
            .spawn(move || {
                let started = std::time::Instant::now();
                let loaded = CharScorer::load(&path).and_then(|scorer| {
                    scorer.score("今天", &["的"])?;
                    Ok(Box::new(scorer) as Box<dyn SentenceScorer>)
                });
                if loaded.is_ok() {
                    tracing::info!(
                        path = %path.display(),
                        total_ms = started.elapsed().as_millis(),
                        "知微已加载并预热"
                    );
                }
                let _ = tx.send(loaded);
            });
        match spawned {
            Ok(_) => {
                self.word_loader = Some(rx);
                self.rescore.watch_loading();
            }
            Err(error) => tracing::warn!(%error, "起不了知微加载线程，不按前文排词"),
        }
    }

    /// 知微加载有结果了就接上。返回是否还在加载。
    pub(super) fn attach_loaded_word_model(&mut self) -> bool {
        let Some(rx) = &self.word_loader else {
            return false;
        };
        match rx.try_recv() {
            Ok(Ok(scorer)) => {
                self.engine.set_async_word_scorer(Some(scorer));
                self.word_loader = None;
                false
            }
            Ok(Err(error)) => {
                tracing::warn!(%error, "知微加载失败，不按前文排词");
                self.word_loader = None;
                false
            }
            Err(TryRecvError::Empty) => true,
            Err(TryRecvError::Disconnected) => {
                self.word_loader = None;
                false
            }
        }
    }

    /// 卸掉知微（配置切回 tongbian 或关掉本地模型）。
    pub(super) fn unload_word_model(&mut self) {
        if self.word_loader.take().is_some() || self.engine.has_word_scorer() {
            tracing::info!("知微已卸载");
        }
        self.engine.set_async_word_scorer(None);
    }
}
```

`host/model/mod.rs`：

1. `mod rescore_monitor;` 后加 `mod word_model;`。
2. `load_local_model` 第一行（`if self.model_loader.is_some() || …` 之前）加 `self.load_word_model();`。
   `[model]` 变了（`host/config/mod.rs:98`）会再进这里：知微那边自己有「在加载 / 已接上就不再加载」与「不要就卸」的判断，不会加载两次。
3. `attach_loaded_model` 开头改成：

```rust
    pub fn attach_loaded_model(&mut self) {
        let word_loading = self.attach_loaded_word_model();
        let Some(rx) = &self.model_loader else {
            if !word_loading {
                self.rescore.stop_watching();
            }
            return;
        };
```

   其余三处 `self.rescore.stop_watching();` 改成 `if !word_loading { self.rescore.stop_watching(); }`。
4. `model_loading`：`self.model_loader.is_some() || self.word_loader.is_some()`。
5. `unload_local_model` 第一行加 `self.unload_word_model();`。

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && cargo clippy -p qingjian-macos --all-targets -- -D warnings
```

- [ ] **Step 3：装到本机，量内存（真人）**

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && apps/macos/scripts/bundle.sh --install
```

给审计 / 用户的步骤：

1. 切到青简，在任意文本框打几个字，等 5 秒（模型加载完）。
2. 终端跑 `ps -o rss=,comm= -p "$(pgrep -f 'Qingjian.app/Contents/MacOS' | head -1)"`，记下 RSS（KB）。
3. 在 `~/Library/Application Support/Qingjian/config.toml` 的 `[model]` 下加一行 `scorers = "tongbian"`，保存（输入法热加载）；再打几个字，等 5 秒，再跑第 2 步。
4. 把两个数和 `~/Library/Logs/Qingjian/` 里最新日志中「知微已加载并预热 total_ms=」「知微已卸载」两行一起报回；把配置改回 `both`。

两个数（换算成 MB）与 total_ms 填进 Task 5 行。

- [ ] **Step 4：`tongbian` 与现状一致**

CLI 没有 `[model]`，不给 `--word-model` 就是 tongbian；Task 4 收尾的**对照**列与 Task 3 的数字相同即是证据，写进 Task 5 行。
macOS 上 `scorers = "tongbian"` 时日志有「知微已卸载」、之后不再出现「知微已加载」；恢复 `both` 后出现。

- [ ] **Step 5：文档、提交**

`cloud/docs/design.md` 第 3 节末尾加一小节（Task 7 还会往这里补 Tab 的说明）：

```markdown
### 3.x 上下文预测：按前文排词与本地续写（2026-10-04）

`[model] scorers = "both"`（素笺缺省）同时加载两个随包小模型：含章·通变照旧按按键给整句路径重排与生成；
含章·知微按光标前文给第一页词级候选打分重排，并在停顿后续写几个字挂在拼音右侧。常驻内存多约 56 MB（实测见计划文件）。
`"tongbian"` 即上游行为。设计与评测：`specs/2026-10-04-context-prediction-design.md`、`plans/2026-10-04-context-prediction.md`。
```

fork-patch 加：

```markdown
| `crates/qingjian-platform/src/config/scorer_set.rs` | 新文件 | `ScorerSet` |
| `crates/qingjian-platform/src/config/model.rs`、`mod.rs` | 加 1 个字段、2 行模板、1 条测试 | `[model] scorers` |
| `apps/macos/src/host/model/word_model.rs` | 新文件 | 知微后台加载、接上、卸掉 |
| `apps/macos/src/host/model/mod.rs` | 加 1 行 mod、挂钩 4 处 | 与通变一起加载 / 接上 / 卸掉 |
| `apps/macos/src/host/mod.rs`、`init.rs` | 加 1 个字段 | `word_loader` |
```

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && cargo fmt --all && git add -A crates/qingjian-platform apps/macos cloud/docs && git commit -m "feat(macos): [model] scorers = both，同时加载通变与知微

- 通变照旧排整句；知微接到 Engine 的词级重排上，切回 tongbian 真卸掉
- 内存实测见计划文件"
```

---

## Task 6：续写接口 `continue_text` 与 `--eval-continuation`

**Files:**
- Create: `crates/qingjian-neural/src/continuation.rs`
- Modify: `crates/qingjian-neural/src/lib.rs`（`mod continuation;`）
- Modify: `crates/qingjian-core/src/sentence/scorer.rs`（trait 加缺省方法）
- Modify: `crates/qingjian-neural/src/core_scorer.rs`（`CharScorer` 实现它）
- Create: `apps/cli/src/eval/continuation.rs`
- Modify: `apps/cli/src/eval/mod.rs`、`args.rs`、`main.rs`

- [ ] **Step 1：写失败的测试（随包模型存在才跑，不在就打印跳过）**

`crates/qingjian-neural/src/continuation.rs` 先只写文件头与测试：

```rust
//! 本地续写：知微贪心解码，接着前文往下写几个字，给 Tab 接受用。
//! 素笺分叉，见 cloud/docs/specs/2026-10-04-context-prediction-design.md。

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::CharScorer;

    fn scorer() -> Option<CharScorer> {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/models/hanzhang-zhiwei");
        dir.exists().then(|| CharScorer::load(&dir).unwrap())
    }

    #[test]
    fn continues_a_few_characters_and_stops_at_punctuation() {
        let Some(scorer) = scorer() else {
            eprintln!("没有知微模型，跳过");
            return;
        };
        let (text, average) = scorer
            .continue_text("今天下午我们开会讨论输入法的", 8)
            .unwrap()
            .expect("有续写");
        eprintln!("续写：{text}（{average:.2}）");
        assert!(!text.is_empty() && text.chars().count() <= 8, "{text}");
        assert!(text.chars().all(|c| !super::STOPS.contains(&c)), "{text}");
        assert!(average <= 0.0 && average > -20.0, "{average}");
        let (one, _) = scorer
            .continue_text("今天下午我们开会讨论输入法的", 1)
            .unwrap()
            .unwrap();
        assert_eq!(one.chars().count(), 1);
        assert_eq!(scorer.continue_text("", 8).unwrap(), None);
        assert_eq!(scorer.continue_text("   ", 8).unwrap(), None);
        assert_eq!(scorer.continue_text("今天", 0).unwrap(), None);
        let long = "很长的前文。".repeat(40);
        assert!(scorer.continue_text(&long, 4).is_ok());
    }
}
```

`lib.rs` 加 `mod continuation;`。

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && cargo test -p qingjian-neural continuation 2>&1 | head
```

Expected: `no method named continue_text`。

- [ ] **Step 2：实现**

`continuation.rs` 测试模块之前加：

```rust
use candle_core::Tensor;

use crate::vocab::EOS;
use crate::{CharScorer, NeuralError};

/// 续写遇到这些就停：句读标点与换行，再写就是下一句了。
pub(crate) const STOPS: &[char] = &[
    '，', '。', '！', '？', '；', '：', '、', '\n', ',', '.', '!', '?', ';',
];

impl CharScorer {
    /// 接着 `before` 续写最多 `max_chars` 个字（贪心），返回续写文本与每字平均 log 概率；
    /// `before` 空白、`max_chars` 为 0、或第一个字就是标点 / 句尾时返回 `None`。
    /// 前文过长从左截到模型上下文给续写留出位置；序列开头保留 `<eos>`（训练时每行末尾补它，当句首用）。
    pub fn continue_text(
        &self,
        before: &str,
        max_chars: usize,
    ) -> Result<Option<(String, f64)>, NeuralError> {
        if before.trim().is_empty() || max_chars == 0 {
            return Ok(None);
        }
        let limit = self.model().config().context;
        let mut ids: Vec<u32> = self.vocab().encode(before);
        let room = limit.saturating_sub(max_chars + 1).max(1);
        if ids.len() > room {
            ids.drain(..ids.len() - room);
        }
        ids.insert(0, EOS);
        let device = self.model().device();
        let split = ids.len() - 1;
        let mut cache = self.model().prefix_cache(&ids[..split])?;
        let mut last = Tensor::from_vec(vec![ids[split]], (1, 1), device)?;
        let mut out: Vec<u32> = Vec::new();
        let mut total = 0.0f64;
        for _ in 0..max_chars {
            let (log_probs, grown) = self.model().step(&cache, &last)?;
            let row = log_probs.squeeze(0)?.to_vec1::<f32>()?;
            let Some((token, lp)) = row
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .map(|(token, lp)| (token as u32, f64::from(*lp)))
            else {
                break;
            };
            let text = self.vocab().decode(&[token]);
            if token == EOS || text.starts_with('<') || text.chars().any(|c| STOPS.contains(&c)) {
                break;
            }
            out.push(token);
            total += lp;
            cache = grown;
            last = Tensor::from_vec(vec![token], (1, 1), device)?;
        }
        if out.is_empty() {
            return Ok(None);
        }
        let text = self.vocab().decode(&out);
        Ok(Some((text, total / out.len() as f64)))
    }
}
```

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && cargo test -p qingjian-neural continuation -- --nocapture
```

Expected: 通过，打出来的续写像中文。

- [ ] **Step 3：Core 的 trait 加缺省方法，`CharScorer` 实现**

`crates/qingjian-core/src/sentence/scorer.rs` 的 `generate` 后加：

```rust
    /// 接着 `before` 往下写最多 `max_chars` 个字：(文本, 每字平均 log 概率)。做不了（P2C 模型、模型出错）返回 `None`。
    /// 素笺的本地续写用，见 `engine/prediction/local_continuation.rs`。
    fn continue_text(&self, _before: &str, _max_chars: usize) -> Option<(String, f64)> {
        None
    }
```

`crates/qingjian-neural/src/core_scorer.rs` 的 `impl SentenceScorer for CharScorer` 里加：

```rust
    fn continue_text(&self, before: &str, max_chars: usize) -> Option<(String, f64)> {
        match CharScorer::continue_text(self, before, max_chars) {
            Ok(result) => result,
            Err(error) => {
                tracing::warn!(%error, "本地续写失败，本次不用");
                None
            }
        }
    }
```

- [ ] **Step 4：`--eval-continuation`**

`apps/cli/src/eval/continuation.rs`：

```rust
//! 本地续写评测：在留出文本上每隔若干字取一个位置，前文给模型、紧接着的几个字当真值，
//! 看续写的头两个字对不对（代理精度），并按平均 log 概率的门槛分档，挑让精度 ≥60% 的 τ。

use std::fmt;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use qingjian_neural::CharScorer;

use crate::eval::EvalError;
use crate::latency::Latencies;

/// 每隔多少字取一个位置。
pub const STRIDE: usize = 20;

/// 前文给多少字（与壳读的前文一致）。
pub const BEFORE_CHARS: usize = 64;

/// 续写长度上限（与 Core 的 `CONTINUATION_MAX_CHARS` 一致）。
pub const MAX_CHARS: usize = 8;

/// 真值至少几个字；代理精度算续写与真值的共同前缀至少几个字。
pub const MIN_TRUTH: usize = 2;
pub const MATCH_CHARS: usize = 2;

/// 报告里分档的门槛 τ（平均 log 概率 ≥ τ 才显示）。
pub const THRESHOLDS: [f64; 7] = [-0.5, -1.0, -1.5, -2.0, -2.5, -3.0, f64::NEG_INFINITY];

/// 一个评测位置。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sample {
    pub before: String,
    pub truth: String,
}

/// 一个位置的续写结果。
#[derive(Debug, Clone)]
pub struct Outcome {
    pub average: f64,
    pub length: usize,
    pub correct: bool,
}

/// 从文本里取位置：每行独立；位置前要是汉字，真值是紧接着的汉字（遇非汉字停，最多 [`MAX_CHARS`] 个）且至少 [`MIN_TRUTH`] 个。
pub fn samples(text: &str) -> Vec<Sample> {
    let mut out = Vec::new();
    for line in text.lines() {
        let chars: Vec<char> = line.chars().collect();
        let mut position = STRIDE;
        while position + MIN_TRUTH <= chars.len() {
            let before: String = chars[position.saturating_sub(BEFORE_CHARS)..position]
                .iter()
                .collect();
            let truth: String = chars[position..]
                .iter()
                .take_while(|c| is_han(**c))
                .take(MAX_CHARS)
                .collect();
            if before.chars().last().is_some_and(is_han) && truth.chars().count() >= MIN_TRUTH {
                out.push(Sample { before, truth });
            }
            position += STRIDE;
        }
    }
    out
}

fn is_han(c: char) -> bool {
    matches!(c as u32, 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xF900..=0xFAFF | 0x20000..=0x323AF)
}

/// 续写与真值的共同前缀是否够长。
pub fn matches(continued: &str, truth: &str) -> bool {
    continued
        .chars()
        .zip(truth.chars())
        .take_while(|(a, b)| a == b)
        .count()
        >= MATCH_CHARS
}

#[derive(Debug, Default)]
pub struct ContinuationReport {
    pub samples: usize,

    /// 模型写出来了的位置数。
    pub produced: usize,

    pub outcomes: Vec<Outcome>,

    /// 每个位置一次 `continue_text` 的耗时（写没写出来都算）。
    pub latency: Latencies,
}

impl ContinuationReport {
    /// 门槛 τ 下显示的条数与其中正确的条数。
    pub fn at(&self, threshold: f64) -> (usize, usize) {
        let shown: Vec<&Outcome> = self
            .outcomes
            .iter()
            .filter(|o| o.average >= threshold)
            .collect();
        let correct = shown.iter().filter(|o| o.correct).count();
        (shown.len(), correct)
    }

    pub fn p90(&self) -> Duration {
        self.latency.quantile(0.9)
    }
}

impl fmt::Display for ContinuationReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "本地续写评测（代理精度 = 续写与真值共同前缀 ≥ {MATCH_CHARS} 字）")?;
        let mean_length = if self.outcomes.is_empty() {
            0.0
        } else {
            self.outcomes.iter().map(|o| o.length).sum::<usize>() as f64 / self.outcomes.len() as f64
        };
        writeln!(
            f,
            "位置 {}  写出 {}  平均长度 {mean_length:.1} 字  时延 p50 {:.0} ms / p90 {:.0} ms",
            self.samples,
            self.produced,
            self.latency.quantile(0.5).as_secs_f64() * 1000.0,
            self.p90().as_secs_f64() * 1000.0,
        )?;
        writeln!(f, "{:>8}  {:>8}  {:>8}  {:>8}", "τ", "显示率", "代理精度", "条数")?;
        for threshold in THRESHOLDS {
            let (shown, correct) = self.at(threshold);
            let shown_rate = if self.samples == 0 {
                0.0
            } else {
                shown as f64 * 100.0 / self.samples as f64
            };
            let precision = if shown == 0 {
                0.0
            } else {
                correct as f64 * 100.0 / shown as f64
            };
            let label = if threshold.is_finite() {
                format!("{threshold:.1}")
            } else {
                "不设".to_owned()
            };
            writeln!(f, "{label:>8}  {shown_rate:>7.1}%  {precision:>7.1}%  {shown:>8}")?;
        }
        Ok(())
    }
}

pub fn run(scorer: &CharScorer, paths: &[PathBuf]) -> Result<ContinuationReport, EvalError> {
    let mut report = ContinuationReport::default();
    for path in paths {
        let text = std::fs::read_to_string(path).map_err(|source| EvalError::Read {
            path: path.clone(),
            source,
        })?;
        for sample in samples(&text) {
            report.samples += 1;
            let started = Instant::now();
            let result = scorer.continue_text(&sample.before, MAX_CHARS)?;
            report.latency.push(started.elapsed());
            let Some((continued, average)) = result else {
                continue;
            };
            report.produced += 1;
            report.outcomes.push(Outcome {
                average,
                length: continued.chars().count(),
                correct: matches(&continued, &sample.truth),
            });
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn samples_need_han_before_and_two_han_after() {
        let line: String = format!("{}输入法的候选排序。", "我".repeat(STRIDE));
        let samples = samples(&line);
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].truth, "输入法的候选排序");
        assert!(samples[0].before.ends_with('我'));
        assert!(samples(&format!("{}abc 输入法", "我".repeat(STRIDE))).is_empty());
    }

    #[test]
    fn match_needs_two_common_leading_characters() {
        assert!(matches("输入法", "输入"));
        assert!(!matches("输出", "输入法"));
        assert!(!matches("", "输入"));
    }
}
```

`eval/mod.rs` 加 `pub mod continuation;`。`EvalError::Generate` 的 `#[from] NeuralError` 已有，`?` 直接用。

`args.rs` 在 `eval_context_details` 后加：

```rust
    /// 本地续写评测：在这些文本（一行一段）上每 20 字取一个位置，前文给知微续写、下文当真值；需要 --neural 指向知微
    #[arg(long, num_args = 1.., requires = "neural")]
    pub eval_continuation: Vec<PathBuf>,
```

`main.rs` 在 `--eval-context` 分支之前加：

```rust
    if !args.eval_continuation.is_empty() {
        let path = args.neural.as_ref().expect("required by clap");
        let scorer = qingjian_neural::CharScorer::load(path)?;
        if scorer.vocab().sep().is_some() {
            return Err(qingjian_neural::NeuralError::Corrupt(
                "--eval-continuation expects Hanzhang Zhiwei (a character LM without <sep>)",
            )
            .into());
        }
        let report = eval::continuation::run(&scorer, &args.eval_continuation)?;
        print!("{report}");
        return Ok(());
    }
```

- [ ] **Step 5：跑评测，选 τ；p90 > 150 ms 停下报告**

留出文本用用户文档的正文（模型训练语料是公开网页语料，不含它们）：

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && cargo test -p qingjian-cli continuation && cargo run --release -p qingjian-cli -- --neural data/models/hanzhang-zhiwei --eval-continuation $(find docs/user -name '*.md' | sort)
```

挑**代理精度 ≥ 60% 里显示率最高**的 τ，记在下面，Task 7 的 `CONTINUATION_THRESHOLD` 用它。
**p90 时延超过 150 ms 就停下**，把整张表发审计，不自行放宽时限。

| τ | 显示率 | 代理精度 | p50 / p90 ms |
|---|---|---|---|
| | | | |

- [ ] **Step 6：fork-patch、提交**

```markdown
| `crates/qingjian-neural/src/continuation.rs` | 新文件 | `CharScorer::continue_text` 贪心续写 |
| `crates/qingjian-neural/src/lib.rs` | 加 1 行 | `mod continuation;` |
| `crates/qingjian-neural/src/core_scorer.rs` | 加 1 个方法 | `SentenceScorer::continue_text` 的实现 |
| `crates/qingjian-core/src/sentence/scorer.rs` | 加 1 个缺省方法 | `continue_text` |
| `apps/cli/src/eval/continuation.rs` | 新文件 | `--eval-continuation` |
| `apps/cli/src/eval/mod.rs`、`args.rs`、`main.rs` | 各加几行 | 挂进去 |
```

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && cargo fmt --all && git add -A crates/qingjian-neural crates/qingjian-core apps/cli cloud/docs && git commit -m "feat(neural): 知微贪心续写 continue_text 与 --eval-continuation

- 遇句读标点 / <eos> / 长度上限停，返回文本与每字平均 log 概率
- 评测在留出文本上报告各门槛下的显示率、代理精度与时延；τ 见计划文件"
```

---

## Task 7：macOS 显示与 Tab 接受

**Files:**
- Modify: `crates/qingjian-core/src/engine/input_log/source.rs`（`LocalContinuation` 变体）、`engine/learning/mod.rs:84-85`（计词数）
- Modify: `apps/cli/src/replay/report.rs:65-69`（新变体归不评的来源）
- Modify: `crates/qingjian-core/src/engine/rescoring/worker.rs`（Job / Scored 各加 1 个字段、两类任务各留最新一条、`submit_continuation`）
- Modify: `crates/qingjian-core/src/engine/rescoring/mod.rs`（1 个常数）、`word_rescore/mod.rs`（收续写）
- Create: `crates/qingjian-core/src/engine/rescoring/worker_tests.rs`
- Create: `crates/qingjian-core/src/engine/prediction/local_continuation/mod.rs`、`tests.rs`
- Modify: `crates/qingjian-core/src/engine/prediction/mod.rs`（`mod` + 导出）、`engine/mod.rs`（3 个字段）、`composing.rs`（`clear` 里 1 行）
- Create: `apps/macos/src/host/model/continuation.rs`
- Modify: `apps/macos/src/host/mod.rs`、`init.rs`（`sentence_local` 字段）、`host/model/mod.rs`（3 处）、`host/cloud/mod.rs:138`、
  `imk/controller/display.rs`（refresh 清旧续写；`accept_sentence` 分本地 / 云端）
- Create: `cloud/scripts/tab-continuation-e2e.sh`
- Modify: `cloud/docs/design.md`（Tab 行为）、`fork-patch.md`

接受的形状：显示上与云端整句补全一样（`首选 + 续写` 挂在拼音右侧，Tab 接受），**上屏与记账分开走**：首选照常 `commit`
（词频、同输入串 choice、转移都记，日志来源 `Word`），续写文本紧接着上屏、切词记个人 n-gram，日志另记一条来源
`LocalContinuation`（键为空），回放就能分别统计本地续写与云端补全的接受率。只在首选**盖住整段拼音**时才续写。

- [ ] **Step 1：日志来源 `LocalContinuation`**

`crates/qingjian-core/src/engine/input_log/source.rs` 的 `CloudSentence,` 后加：

```rust
    /// Tab 接受的本地续写里续写的那部分（首选本身另记一条 `Word`）。素笺分叉。
    LocalContinuation,
```

`engine/learning/mod.rs:85` 的 `InputSource::Sentence | InputSource::CloudSentence => {` 改成
`InputSource::Sentence | InputSource::CloudSentence | InputSource::LocalContinuation => {`。
`apps/cli/src/replay/report.rs:65-69` 的 `None` 分支加 `| InputSource::LocalContinuation`（不评、计入「不评的来源」表，接受率从那里看）。

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && cargo check --workspace --all-targets
```

- [ ] **Step 2：后台线程：两类任务各留最新一条，会续写。先写测试**

`rescoring/worker_tests.rs`：

```rust
//! 后台线程的排队规则：打分与续写各留最新一条，互不顶掉。

use std::time::{Duration, Instant};

use crate::engine::rescoring::RescoreWorker;
use crate::sentence::SentenceScorer;

/// 慢打分器：每次打分睡 30 ms，让后面的任务排起队。
struct Slow;

impl SentenceScorer for Slow {
    fn score(&self, _context: &str, _keys: &str, texts: &[&str]) -> Vec<f64> {
        std::thread::sleep(Duration::from_millis(30));
        texts.iter().map(|_| -1.0).collect()
    }

    fn continue_text(&self, before: &str, _max_chars: usize) -> Option<(String, f64)> {
        Some((format!("{before}续"), -0.5))
    }
}

#[test]
fn a_queued_continuation_is_not_dropped_by_a_newer_scoring_job() {
    let worker = RescoreWorker::spawn(Box::new(Slow));
    worker.submit("前文".into(), "a".into(), vec!["甲".into()], None);
    // 线程在算第一条时这两条排着队：旧规则只留最新一条（打分 b），续写就丢了
    worker.submit_continuation("前文甲".into(), 4);
    worker.submit("前文".into(), "b".into(), vec!["乙".into()], None);
    let started = Instant::now();
    let mut keys = Vec::new();
    let mut continued = None;
    while (keys.len() < 2 || continued.is_none()) && started.elapsed() < Duration::from_secs(5) {
        if let Some(scored) = worker.poll() {
            if let Some(result) = scored.continued {
                continued = Some(result);
            } else {
                keys.push(scored.keys);
            }
        } else {
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    assert_eq!(
        continued,
        Some(("前文甲".to_owned(), Some(("前文甲续".to_owned(), -0.5))))
    );
    // 最新的打分一定在；a 可能已经开算了也在，但不会多于两条
    assert!(keys.contains(&"b".to_owned()) && keys.len() <= 2, "{keys:?}");
}
```

`rescoring/mod.rs` 的 `#[cfg(test)] mod tests;` 旁加 `#[cfg(test)] mod worker_tests;`。

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && cargo test -p qingjian-core worker_tests 2>&1 | head
```

Expected: 编译错误（`submit_continuation` / `continued` 不存在）。

- [ ] **Step 3：实现 worker**

`rescoring/worker.rs`：`Job` 加字段 `continue_from: Option<(String, usize)>`（续写的前文与长度上限；`None` 不续），
`Scored` 加字段 `pub continued: Option<(String, Option<(String, f64)>)>`（续写用的前文与结果，超时或写不出来是 `None`）。
`use super::{GENERATE_BEAM, GENERATE_MAX_CHARS};` 改成 `use super::{CONTINUATION_TIME_LIMIT, GENERATE_BEAM, GENERATE_MAX_CHARS};`。

线程循环改成「两类各留最新一条」，原来的循环体搬进 `run_job`：

```rust
            .spawn(move || {
                while let Ok(job) = job_rx.recv() {
                    // 攒了好几条：打分与续写各留最新一条，两类任务不互相顶掉（续写是打分完成后才发的）
                    let mut scoring: Option<Job> = None;
                    let mut continuation: Option<Job> = None;
                    for pending in std::iter::once(job).chain(std::iter::from_fn(|| job_rx.try_recv().ok())) {
                        if pending.continue_from.is_some() {
                            continuation = Some(pending);
                        } else {
                            scoring = Some(pending);
                        }
                    }
                    for job in scoring.into_iter().chain(continuation) {
                        if result_tx.send(run_job(&*scorer, job)).is_err() {
                            return;
                        }
                    }
                }
            })
```

```rust
/// 跑一条任务：打分、可选的生成、可选的续写。
fn run_job(scorer: &dyn SentenceScorer, job: Job) -> Scored {
    let texts: Vec<&str> = job.texts.iter().map(String::as_str).collect();
    let scores = if texts.is_empty() {
        Vec::new()
    } else {
        let started = std::time::Instant::now();
        let scores = scorer.score(&job.context, &job.keys, &texts);
        tracing::debug!(
            texts = texts.len(),
            context_chars = job.context.chars().count(),
            keys = job.keys.len(),
            ms = started.elapsed().as_millis(),
            "神经重打分完成"
        );
        scores
    };
    let generated = job.generate.map(|keys| {
        let started = std::time::Instant::now();
        let texts = scorer.generate(&keys, GENERATE_BEAM, GENERATE_MAX_CHARS);
        tracing::debug!(
            keys = keys.len(),
            got = texts.len(),
            ms = started.elapsed().as_millis(),
            "整句生成完成"
        );
        (keys, texts)
    });
    let continued = job.continue_from.map(|(before, max_chars)| {
        let started = std::time::Instant::now();
        let result = scorer.continue_text(&before, max_chars);
        let elapsed = started.elapsed();
        tracing::debug!(
            before_chars = before.chars().count(),
            got = result.as_ref().map(|(t, _)| t.chars().count()),
            ms = elapsed.as_millis(),
            "本地续写完成"
        );
        // 太慢的这次不用：用户早就敲下一个键了
        let result = (elapsed <= CONTINUATION_TIME_LIMIT).then_some(result).flatten();
        (before, result)
    });
    Scored {
        context: job.context,
        keys: job.keys,
        texts: job.texts,
        scores,
        generated,
        continued,
    }
}
```

`submit` 里 `Job { …, generate, continue_from: None }`；加方法：

```rust
    /// 只要续写：空的打分任务带一段前文。
    pub fn submit_continuation(&self, before: String, max_chars: usize) {
        if self
            .jobs
            .send(Job {
                context: String::new(),
                keys: String::new(),
                texts: Vec::new(),
                generate: None,
                continue_from: Some((before, max_chars)),
            })
            .is_err()
        {
            tracing::warn!("神经重打分线程已退出");
        }
    }
```

`rescoring/mod.rs` 常数区加：

```rust
/// 单次本地续写最多等多久，超过就放弃这次（见 prediction/local_continuation）。
pub(super) const CONTINUATION_TIME_LIMIT: std::time::Duration = std::time::Duration::from_millis(150);
```

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && cargo test -p qingjian-core rescoring
```

Expected: 全绿（含上游 `rescoring::tests` 与新 `worker_tests`）。

- [ ] **Step 4：Core 的续写入口。先写测试 `prediction/local_continuation/tests.rs`**

```rust
//! 本地续写的 Engine 级测试：假知微，不依赖模型文件。

use std::collections::HashMap;
use std::time::{Duration, Instant};

use qingjian_dictionary::Dictionary;

use crate::candidate::{Candidate, CandidateKind};
use crate::engine::{Engine, InputLogEntry, InputLogger, InputSource, Learner};
use crate::sentence::SentenceScorer;

/// 假知微：续写固定文本，平均分可调。
struct Writes(&'static str, f64);

impl SentenceScorer for Writes {
    fn score(&self, _context: &str, _keys: &str, texts: &[&str]) -> Vec<f64> {
        texts.iter().map(|_| -1.0).collect()
    }

    fn continue_text(&self, _before: &str, _max_chars: usize) -> Option<(String, f64)> {
        Some((self.0.to_owned(), self.1))
    }
}

/// 只数 choice。
struct Choices(HashMap<String, u32>);

impl Learner for Choices {
    fn record(&mut self, _candidate: &Candidate) {}

    fn weight(&self, _text: &str) -> u32 {
        0
    }

    fn record_choice(&mut self, input: &str, text: &str) {
        *self.0.entry(format!("{input}\t{text}")).or_default() += 1;
    }

    fn choice_weight(&self, input: &str, text: &str) -> u32 {
        self.0.get(&format!("{input}\t{text}")).copied().unwrap_or(0)
    }
}

/// 把日志条目攒在内存里。
struct Memory(std::sync::Arc<std::sync::Mutex<Vec<InputLogEntry>>>);

impl InputLogger for Memory {
    fn record(&mut self, entry: InputLogEntry) {
        self.0.lock().unwrap().push(entry);
    }
}

fn engine(scorer: Writes) -> Engine {
    Engine::new(Dictionary::parse("邮箱\tyou xiang\t9000\n邮\tyou\t900\n").unwrap())
        .with_async_word_scorer(Box::new(scorer), None)
}

fn wait(engine: &mut Engine) {
    let started = Instant::now();
    while !engine.poll_rescoring() {
        assert!(started.elapsed() < Duration::from_secs(5), "后台没回结果");
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn first(engine: &Engine) -> Candidate {
    engine.query().unwrap().candidates.items[0].clone()
}

#[test]
fn continuation_is_first_candidate_plus_text() {
    let mut engine = engine(Writes("地址", -0.5));
    engine.set_input("youxiang");
    let first = first(&engine);
    assert_eq!(first.text, "邮箱");
    assert!(engine.request_continuation(&first));
    // 同一条件不重复发
    assert!(!engine.request_continuation(&first));
    wait(&mut engine);
    assert_eq!(engine.take_continuation().as_deref(), Some("邮箱地址"));
    assert_eq!(engine.take_continuation(), None);
}

#[test]
fn low_confidence_continuation_is_dropped() {
    let mut engine = engine(Writes("地址", -5.0));
    engine.set_input("youxiang");
    let first = first(&engine);
    assert!(engine.request_continuation(&first));
    wait(&mut engine);
    assert_eq!(engine.take_continuation(), None);
}

#[test]
fn a_first_candidate_that_leaves_pinyin_behind_is_not_continued() {
    let mut engine = engine(Writes("地址", -0.5));
    engine.set_input("youxiangd");
    let candidate = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|c| c.text == "邮箱" && c.kind == CandidateKind::Chinese)
        .expect("有 邮箱");
    assert!(!engine.request_continuation(&candidate));
}

#[test]
fn a_new_key_discards_the_old_continuation() {
    let mut engine = engine(Writes("地址", -0.5));
    engine.set_input("youxiang");
    let first = first(&engine);
    assert!(engine.request_continuation(&first));
    engine.cancel_continuation();
    wait(&mut engine);
    assert_eq!(engine.take_continuation(), None);
}

#[test]
fn private_input_never_continues() {
    let mut engine = engine(Writes("地址", -0.5));
    engine.set_private(true);
    engine.set_input("youxiang");
    let first = first(&engine);
    assert!(!engine.request_continuation(&first));
}

#[test]
fn accepting_commits_the_word_normally_and_logs_the_tail_separately() {
    let log = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let mut engine = engine(Writes("地址", -0.5))
        .with_learner(Box::new(Choices(HashMap::new())))
        .with_input_logger(Box::new(Memory(log.clone())));
    engine.set_input("youxiang");
    let first = first(&engine);
    assert!(engine.request_continuation(&first));
    wait(&mut engine);
    assert_eq!(engine.take_continuation().as_deref(), Some("邮箱地址"));
    assert_eq!(engine.accept_continuation().as_deref(), Some("邮箱地址"));
    assert!(engine.composition().is_empty());
    // 首选照常记 choice
    assert_eq!(engine.learner().choice_weight("youxiang", "邮箱"), 1);
    // 日志：一条 Word（邮箱）加一条 LocalContinuation（地址）
    let sources: Vec<(InputSource, String)> = log
        .lock()
        .unwrap()
        .iter()
        .filter_map(|entry| match entry {
            InputLogEntry::Commit(commit) => Some((commit.source, commit.text.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(
        sources,
        [
            (InputSource::Word, "邮箱".to_owned()),
            (InputSource::LocalContinuation, "地址".to_owned()),
        ]
    );
    // 没有显示中的续写时接受什么都不做
    assert_eq!(engine.accept_continuation(), None);
}
```

`prediction/mod.rs` 的 `mod kind;` 后加 `mod local_continuation;`，`pub use` 一行 `pub use local_continuation::{CONTINUATION_MAX_CHARS, CONTINUATION_THRESHOLD};`。
先建 `local_continuation/mod.rs` 只含文件头与 `#[cfg(test)] mod tests;`：

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && cargo test -p qingjian-core local_continuation 2>&1 | head
```

Expected: 编译错误（`request_continuation` 不存在）。`InputLogger` trait 若不在 `crate::engine` 导出里，按 `lib.rs` 的 `pub use engine::{…InputLogger…}` 找到它实际的路径改 `use`。

- [ ] **Step 5：实现 `prediction/local_continuation/mod.rs`**

```rust
//! 本地续写：知微把第一页排完之后，以「前文 + 当前首选」为条件往下写几个字，交给壳挂在拼音右侧、Tab 接受。
//! 显示上与云端整句补全同一形状（首选 + 续写）；接受时首选照常 `commit`（词频、choice、转移），
//! 续写文本紧接着上屏、切词记个人 n-gram、日志另记一条 `InputSource::LocalContinuation`。
//! 请求与结果走知微那条后台线程（`rescoring::word_rescore`），按键回调不等它。
//! 素笺分叉，见 cloud/docs/specs/2026-10-04-context-prediction-design.md。

#[cfg(test)]
mod tests;

use crate::candidate::Candidate;
use crate::engine::{Engine, InputSource};
use crate::sentence;

/// 一次最多续写几个字。
pub const CONTINUATION_MAX_CHARS: usize = 8;

/// 每字平均 log 概率低于它的续写不显示；按 `qingjian-cli --eval-continuation` 代理精度 ≥60% 取（见计划文件 Task 6）。
pub const CONTINUATION_THRESHOLD: f64 = -2.0;

impl Engine {
    /// 壳在第一页重排完成后调；`first` 是当前显示的首选。它要盖住整段拼音（接受后缓冲区会空）才续写，
    /// 否则接受时拼音对不上。发出去了返回 `true`；没接知微、私密输入、同一条件已经请求过都不发。
    pub fn request_continuation(&mut self, first: &Candidate) -> bool {
        let Some(worker) = &self.word_rescorer else {
            return false;
        };
        if self.is_private() || self.composition.is_empty() {
            return false;
        }
        let (consumed, _) = self.consumed_by(first);
        if consumed != self.composition.scope().len() {
            return false;
        }
        let before = format!("{}{}", self.rescoring_context(), first.text);
        if self.continuation_from.as_ref().is_some_and(|(b, _)| *b == before) {
            return false;
        }
        self.continuation_from = Some((before.clone(), first.clone()));
        self.continuation = None;
        self.shown_continuation = None;
        worker.submit_continuation(before, CONTINUATION_MAX_CHARS);
        true
    }

    /// 到了的续写：返回要显示的文本（首选 + 续写，替换整段拼音），并记住它正在显示；取走就没了。
    pub fn take_continuation(&mut self) -> Option<String> {
        let (first, tail) = self.continuation.take()?;
        let text = format!("{}{tail}", first.text);
        self.shown_continuation = Some((first, tail));
        Some(text)
    }

    /// 作废进行中与显示中的续写（换了一键、清空）。
    pub fn cancel_continuation(&mut self) {
        self.continuation_from = None;
        self.continuation = None;
        self.shown_continuation = None;
    }

    /// 用户按 Tab 接受了显示中的续写：首选照常上屏，续写紧接着上屏并记进个人 n-gram；返回要插进应用的文本。
    /// 没有显示中的续写返回 `None`。
    pub fn accept_continuation(&mut self) -> Option<String> {
        let (first, tail) = self.shown_continuation.take()?;
        self.continuation_from = None;
        self.continuation = None;
        let mut text = self.commit(&first);
        self.log_commit("", &tail, InputSource::LocalContinuation);
        self.meter_commit(&tail, InputSource::LocalContinuation, false);
        self.history.record(&tail);
        match sentence::segment_text(&tail, &*self.language_model) {
            Some(clauses) => {
                for (index, words) in clauses.iter().enumerate() {
                    if index > 0 {
                        self.chain.reset();
                    }
                    for word in words {
                        self.record_word(word, &[], 1, false, false);
                    }
                }
                if tail.chars().last().is_some_and(|c| !c.is_alphanumeric()) {
                    self.chain.reset();
                }
            }
            None => self.chain.reset(),
        }
        text.push_str(&tail);
        Some(text)
    }

    /// 后台回了续写：条件还是当前这段才收；低于门槛的不要。
    pub(in crate::engine) fn accept_continued(&mut self, before: &str, result: Option<(String, f64)>) {
        let Some((wanted, first)) = &self.continuation_from else {
            return;
        };
        if wanted != before {
            return;
        }
        self.continuation = result
            .filter(|(_, average)| *average >= CONTINUATION_THRESHOLD)
            .map(|(tail, _)| (first.clone(), tail));
    }
}
```

`engine/mod.rs` 在 `word_weight` 字段后加：

```rust
    /// 进行中的本地续写：(条件前文, 当时的首选)。见 prediction/local_continuation。
    continuation_from: Option<(String, Candidate)>,

    /// 到了、等壳来取的续写：(首选, 续写文本)。
    continuation: Option<(Candidate, String)>,

    /// 正显示着、等 Tab 的续写。
    shown_continuation: Option<(Candidate, String)>,
```

`Engine::new` 加 `continuation_from: None, continuation: None, shown_continuation: None,`。
`composing.rs` 的 `clear()` 里 `self.rescoring_before = None;` 后加 `self.cancel_continuation();`。

`word_rescore/mod.rs` 的 `poll_word_rescoring` 循环体开头加：

```rust
            if let Some((before, result)) = scored.continued {
                self.accept_continued(&before, result);
                updated = true;
                continue;
            }
```

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && cargo test -p qingjian-core local_continuation
```

Expected: 6 个通过。繁体模式下首选经 `commit` 会转繁、续写文本不转，这是已知局限，记进 design.md。

- [ ] **Step 6：macOS 壳**

`apps/macos/src/host/mod.rs` 的 `pub sentence: Option<String>,` 后加：

```rust
    /// `sentence` 是本地续写（Tab 走 `accept_continuation`）还是云端整句补全（走 `accept_prediction`）。
    pub sentence_local: bool,
```

`host/init.rs` 的 `sentence: None,` 旁加 `sentence_local: false,`。

新文件 `apps/macos/src/host/model/continuation.rs`：

```rust
//! 本地续写在壳里的接法：知微把第一页排完（或没什么要排）就以当前首选请求续写，结果到了挂到 preedit 右侧——
//! 与云端整句补全同一个位置、同一个 Tab。本地先到先显示，云端只在本地没有时显示。
//! 素笺分叉，见 cloud/docs/fork-patch.md。

use crate::host::Host;

impl Host {
    /// 第一页重排完、重画之后：以当前首选请求续写，开始轮询。用户翻过页、动过高亮、在翻译时不打扰。
    pub(super) fn request_local_continuation(&mut self) {
        if self.session.page != 0 || self.session.navigated || self.translation.is_some() {
            return;
        }
        let Some(first) = self.session.layout.local().first().cloned() else {
            return;
        };
        if self.engine.request_continuation(&first) {
            self.rescore.start_polling();
        }
    }

    /// 轮询里收到续写：挂上去重画。返回是否有续写。
    pub(super) fn show_local_continuation(&mut self) -> bool {
        let Some(text) = self.engine.take_continuation() else {
            return false;
        };
        if self.engine.composition().is_empty() {
            return false;
        }
        tracing::debug!(%text, "本地续写已显示");
        self.sentence = Some(text);
        self.sentence_local = true;
        self.render();
        true
    }
}
```

`host/model/mod.rs`：

1. `mod rescore_monitor;` 旁加 `mod continuation;`。
2. `schedule_rescoring`：条件改成 `if self.engine.rescoring_pending() || self.engine.has_word_scorer()`。
3. `start_rescoring`：

```rust
        if self.engine.request_rescoring() {
            self.rescore.start_polling();
        } else {
            // 没什么要打分（都在缓存里）：直接续写
            self.request_local_continuation();
        }
```

4. `poll_rescoring` 的 `self.rescore.stop();` 之后、`if self.session.page != 0 …` 之前加：

```rust
        if self.show_local_continuation() {
            return;
        }
```

   末尾 `self.render();` 之后加 `self.request_local_continuation();`。

`host/cloud/mod.rs:138` 的 `self.sentence = prediction.sentence;` 改成：

```rust
        // 本地续写先到先显示，云端整句只在本地没有时显示（素笺）
        if self.sentence.is_none() {
            self.sentence = prediction.sentence;
            self.sentence_local = false;
        }
```

`imk/controller/display.rs`：

`refresh` 里 `h.engine.set_rescoring_context(before);` 那个 `if` 之后加：

```rust
            // 上一键的续写对这一键无效（云联想关着时没人清它）
            h.sentence = None;
            h.sentence_local = false;
            h.engine.cancel_continuation();
```

`accept_sentence` 改成：

```rust
    /// 接受拼音右侧的整句：本地续写走 `accept_continuation`（首选照常记账），云端补全走 `accept_prediction`。没有就返回 false。
    pub(super) fn accept_sentence(&self, client: TextClient<'_>) -> bool {
        let Some((text, local)) = host::with(|h| {
            let local = std::mem::take(&mut h.sentence_local);
            h.sentence.take().map(|text| (text, local))
        })
        .flatten() else {
            return false;
        };
        let committed = host::with(|h| {
            if local {
                h.engine.accept_continuation()
            } else {
                Some(h.engine.accept_prediction(&text))
            }
        })
        .flatten()
        .unwrap_or(text);
        tracing::debug!(%committed, local, "接受整句补全");
        client.insert_text(&committed);
        self.refresh(client);
        true
    }
```

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && cargo clippy -p qingjian-macos --all-targets -- -D warnings && apps/macos/scripts/bundle.sh --install
```

- [ ] **Step 7：端到端脚本 `cloud/scripts/tab-continuation-e2e.sh`（先预置前文）**

```bash
#!/usr/bin/env bash
# 本地续写的端到端：往 TextEdit 预置一段前文、光标放到末尾，再敲拼音，停 0.6 秒等续写出现，按 Tab 接受，读回文档内容。
# 测的是宿主前文（应用里已有的文字），不是会话链。
# 用法：cloud/scripts/tab-continuation-e2e.sh "汽车" "youxiang"   （预置的前文、要续写的拼音）
# 前提：青简是当前输入法、中文模式；TextEdit 已打开一个文档（内容会被覆盖）。
set -euo pipefail
before="${1:?前文}"
pinyin="${2:?拼音}"
osascript <<EOF
tell application "TextEdit"
  activate
  set text of document 1 to "$before"
end tell
delay 0.3
tell application "System Events"
  key code 125 using command down
  delay 0.2
  keystroke "$pinyin"
  delay 0.6
  key code 48
  delay 0.3
  keystroke return
end tell
delay 0.2
tell application "TextEdit" to get text of document 1
EOF
```

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && chmod +x cloud/scripts/tab-continuation-e2e.sh && open -a TextEdit && cloud/scripts/tab-continuation-e2e.sh 汽车 youxiang
```

Expected: 输出以「汽车油箱」开头且 油箱 后面带了续写（没带就看 `~/Library/Logs/Qingjian/` 里有没有「本地续写已显示」「本地续写完成 ms=」）。

- [ ] **Step 8：真机验收，记录**

TextEdit：用脚本跑 10 对（从 `cloud/data/eval/context-pairs.tsv` 挑前文长一点的 10 条），记显示次数与接受后是否通顺，填表。
聊天应用（微信 / 飞书任一）：**需要真人**。给审计 / 用户的步骤：

1. 打开聊天输入框（不要发出去），切到青简。
2. 先打一句上下文并让它留在输入框里（例如「今天下午我们开会讨论」），接着打 10 个拼音，每个打完停一秒：
   `youxiang`、`gongshi`、`shiyan`、`jiaoshi`、`yanjiu`、`xingshi`、`tongzhi`、`zhiliao`、`shiji`、`haode`。
3. 每个记两件事：拼音右侧有没有出灰字续写（显示）；按 Tab 后上屏的文字通不通顺（接受后通顺）。
4. 把 10 行「拼音 / 是否显示 / 是否通顺」报回。

| 应用 | 显示 / 10 | 接受后通顺 / 显示 | 备注（哪句不通顺） |
|---|---|---|---|
| TextEdit | | | |
| 聊天应用（名字） | | | |

- [ ] **Step 9：文档（Tab 行为）、fork-patch、提交**

`cloud/docs/design.md` Task 5 那一小节末尾加：

```markdown
**Tab 的行为变了。** 本地续写缺省打开、显示得比云端补全频繁，中文组句时拼音右侧多半有灰字，这时 Tab 是「接受续写」
（首选 + 续写一起上屏），没有灰字时 Tab 仍是翻页；英文模式 Tab 不变。这是用户要的类似 Cursor Tab 的体验，是产品决定。
按分叉约定不改上游用户文档 `docs/user/getting-started/keys.md`，素笺的说明在这里。
已知局限：繁体模式下首选转繁、续写文本不转；接受续写后退格删光重打，只退回首选那部分的学习，续写记进个人 n-gram 的转移不退。
```

fork-patch 加：

```markdown
| `crates/qingjian-core/src/engine/input_log/source.rs` | 加 1 个变体 | `InputSource::LocalContinuation` |
| `crates/qingjian-core/src/engine/learning/mod.rs` | 改 1 行 | 续写按整句计词数 |
| `apps/cli/src/replay/report.rs` | 改 1 行 | 新来源归「不评」 |
| `crates/qingjian-core/src/engine/prediction/local_continuation/mod.rs`、`tests.rs` | 新文件 | 请求 / 取 / 作废 / 接受续写，门槛 τ；测试 |
| `crates/qingjian-core/src/engine/prediction/mod.rs` | 加 2 行 | mod 与导出 |
| `crates/qingjian-core/src/engine/mod.rs` | 加 3 个字段 | `continuation_from` / `continuation` / `shown_continuation` |
| `crates/qingjian-core/src/engine/composing.rs` | 加 1 行 | `clear()` 作废续写 |
| `crates/qingjian-core/src/engine/rescoring/worker.rs` | 加 2 个字段、1 个方法，循环体搬进 `run_job` | 两类任务各留最新一条；会续写，超 150 ms 丢 |
| `crates/qingjian-core/src/engine/rescoring/worker_tests.rs` | 新文件 | 排队规则测试 |
| `crates/qingjian-core/src/engine/rescoring/mod.rs` | 加 1 个常数、1 行 mod | `CONTINUATION_TIME_LIMIT` |
| `crates/qingjian-core/src/engine/rescoring/word_rescore/mod.rs` | 加 5 行 | 收续写 |
| `apps/macos/src/host/model/continuation.rs` | 新文件 | 请求与显示 |
| `apps/macos/src/host/model/mod.rs` | 加 1 行 mod、改 3 处 | 停顿后无事可打也续写；结果到了挂上 |
| `apps/macos/src/host/mod.rs`、`init.rs` | 加 1 个字段 | `sentence_local` |
| `apps/macos/src/host/cloud/mod.rs` | 改 3 行 | 云端整句只在本地没有时显示 |
| `apps/macos/src/imk/controller/display.rs` | 加 3 行、改 `accept_sentence` | 每键清旧续写；Tab 分本地 / 云端 |
| `cloud/scripts/tab-continuation-e2e.sh` | 新文件 | TextEdit 端到端 |
```

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && cargo fmt --all && git add -A crates/qingjian-core apps/macos apps/cli cloud && git commit -m "feat(macos): 本地续写挂到拼音右侧，Tab 接受

- 知微排完第一页后以 前文+首选 续写，首选盖住整段拼音才续；低于 τ 不显示，超 150 ms 丢
- 接受时首选照常 commit（词频 / choice / 转移），续写另记 InputSource::LocalContinuation
- 后台线程打分与续写各留最新一条；本地先到先显示，云端只在本地没有时显示
- 真机：TextEdit 与聊天应用各 10 句，结果见计划文件"
```

---

## Task 8：iOS 桥接前文

**Files:**
- Modify: `cloud/crates/qingjian-cloud-bridge/src/session/mod.rs`（`refresh` 里几行）
- Modify: `cloud/crates/qingjian-cloud-bridge/tests/session.rs`（2 条测试）

桥已经从宿主收 `set_context(before, after)` 存成 `SurroundingText`，只喂给云联想；每键 `refresh()` 之前把 `before` 也给 Engine 的前文入口。
`Engine::clear()` 会清掉前文，所以每次 `refresh` 都设一次；私密输入时不设。桥在 `cloud/` 下，不记 fork-patch。

- [ ] **Step 1：写失败的测试**

`cloud/crates/qingjian-cloud-bridge/tests/session.rs` 末尾加：

```rust
/// 宿主前文进词级排序（素笺上下文预测）：同样的 youxiang，「汽车」后出 油箱。用产品数据，没有就跳过。
#[test]
fn host_context_reorders_candidates() {
    let Some(data) = data_dir() else {
        eprintln!("没有 QINGJIAN_DATA，跳过");
        return;
    };
    let mut session = Session::open(&data, None, None, None).unwrap();
    session.set_context("汽车", "");
    for c in "youxiang".chars() {
        session.push(c);
    }
    assert_eq!(session.entries()[0].text(), "油箱");
    session.clear();
    session.set_context("请把文件发到我的", "");
    for c in "youxiang".chars() {
        session.push(c);
    }
    assert_eq!(session.entries()[0].text(), "邮箱");
    session.clear();
}

/// 私密输入框里不看宿主前文。
#[test]
fn private_input_ignores_host_context() {
    let Some(data) = data_dir() else {
        eprintln!("没有 QINGJIAN_DATA，跳过");
        return;
    };
    let mut session = Session::open(&data, None, None, None).unwrap();
    session.set_private(true);
    session.set_context("汽车", "");
    for c in "youxiang".chars() {
        session.push(c);
    }
    assert_eq!(session.entries()[0].text(), "邮箱");
    session.clear();
}
```

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction/cloud && QINGJIAN_DATA=../data/generated cargo test -p qingjian-cloud-bridge --test session host_context
```

Expected: `host_context_reorders_candidates` 失败（两次首选一样）。

- [ ] **Step 2：实现**

`session/mod.rs` 的 `refresh` 里 `match self.engine.query() {` 之前加：

```rust
        // 宿主前文给词级排序与整句首词（Core 的 query/left_context.rs）；`clear()` 会清掉，每键设一次；私密输入不给
        let before = (!self.engine.is_private())
            .then(|| self.context.as_ref().map(|c| c.before.clone()))
            .flatten();
        self.engine.set_rescoring_context(before);
```

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction/cloud && QINGJIAN_DATA=../data/generated cargo test -p qingjian-cloud-bridge --test session && cargo fmt --all && cargo clippy -p qingjian-cloud-bridge --all-targets -- -D warnings
```

Expected: 全绿。

- [ ] **Step 3：提交**

```bash
cd /Users/liyuqing/sproot/qingjian-context-prediction && git add cloud/crates/qingjian-cloud-bridge && git commit -m "feat(cloud): iOS 桥把宿主前文喂给 Engine 的词级排序，私密输入不给"
```

---

## 收尾

- [ ] 全量检查：`cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace`，`cd cloud && cargo test`。
- [ ] 「评测记录」表填满（产品配置 / 对照两列），每行数字与门槛对照；越线的任务在表里标出并已报告。
- [ ] 每个任务完成时把提交哈希与评测数字发审计会话「素笺输入法」；需要真人的两件事（Task 5 内存、Task 7 聊天应用）按步骤发给它转给用户。
- [ ] 不提交上游 qingjian；不动 `qingjian-mainline` 工作区。
