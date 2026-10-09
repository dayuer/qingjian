//! 学习数据同步的端到端测试：内存里的假服务器按服务端的语义合并增量，输入法一侧用 [`Snapshot`] 读写同格式的文件。
//! 数据都是构造的。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use qingjian_cloud_proto::{LearningPage, LearningPush, LearningRow};

use super::{INBOX, LearningRemote, LearningSync, Snapshot, Table};
use crate::ClientError;

/// 服务器：`(表, 键)` → (合计次数, 最后改动的 seq)；计数表的增量相加，不小于 0。
#[derive(Clone, Default)]
struct FakeServer {
    rows: Arc<Mutex<FakeRows>>,
}

#[derive(Default)]
struct FakeRows {
    seq: u64,

    counts: BTreeMap<(String, String), (i64, u64)>,
}

impl FakeServer {
    fn count(&self, table: &str, key: &str) -> i64 {
        let rows = self.rows.lock().unwrap();
        rows.counts
            .get(&(table.to_owned(), key.to_owned()))
            .map_or(0, |(count, _)| *count)
    }
}

impl LearningRemote for FakeServer {
    fn push_learning(&self, push: &LearningPush) -> Result<u64, ClientError> {
        let mut rows = self.rows.lock().unwrap();
        for change in &push.counts {
            rows.seq += 1;
            let seq = rows.seq;
            let row = rows
                .counts
                .entry((change.table.clone(), change.key.clone()))
                .or_default();
            row.0 = (row.0 + change.delta).max(0);
            row.1 = seq;
        }
        Ok(rows.seq)
    }

    fn learning(&self, since: u64, limit: usize) -> Result<LearningPage, ClientError> {
        let rows = self.rows.lock().unwrap();
        let mut changed: Vec<LearningRow> = rows
            .counts
            .iter()
            .filter(|(_, (_, seq))| *seq > since)
            .map(|((table, key), (count, seq))| LearningRow {
                table: table.clone(),
                key: key.clone(),
                count: *count,
                value: None,
                deleted: false,
                seq: *seq,
            })
            .collect();
        changed.sort_by_key(|row| row.seq);
        changed.truncate(limit);
        Ok(LearningPage {
            rows: changed,
            latest: rows.seq,
        })
    }
}

/// 一台设备的数据目录：`ime/` 是输入法的学习文件，`state/` 是同步的基线与进度。
struct Device {
    root: PathBuf,
}

impl Device {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "qingjian-learning-sync-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("ime")).unwrap();
        Self { root }
    }

    fn ime(&self) -> PathBuf {
        self.root.join("ime")
    }

    fn state(&self) -> PathBuf {
        self.root.join("state")
    }

    fn sync(&self, server: &FakeServer) -> LearningSync {
        LearningSync::open_with(Box::new(server.clone()), &self.ime(), &self.state()).unwrap()
    }

    /// 输入法选了一次这个词。
    fn choose(&self, word: &str) {
        edit_ime(&self.ime(), &format!("user\tadd\t{word}\t1\n"));
    }

    /// 输入法合并收件箱：加上增量、落盘、删文件（与 `FrequencyLearner::merge_inbox_and_flush` 同样的结果）。
    fn merge_inbox(&self) {
        let path = self.ime().join(INBOX);
        if let Ok(inbox) = std::fs::read_to_string(&path) {
            edit_ime(&self.ime(), &inbox);
            std::fs::remove_file(path).unwrap();
        }
    }

    fn count(&self, word: &str) -> i64 {
        user_count(&self.ime(), word)
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn edit_ime(dir: &Path, inbox: &str) {
    let mut snapshot = Snapshot::read_dir(dir).unwrap();
    snapshot.apply_inbox(inbox);
    snapshot.write_ime_dir(dir).unwrap();
}

fn user_count(dir: &Path, word: &str) -> i64 {
    let text = std::fs::read_to_string(dir.join(Table::User.file())).unwrap_or_default();
    text.lines()
        .filter_map(|line| line.split_once('\t'))
        .find(|(text, _)| *text == word)
        .map_or(0, |(_, count)| count.parse().unwrap())
}

/// iOS 上每个键盘扩展进程、每次重建引擎都起一个 `DataSync`，它们共用 App Group 里的同一份基线与进度。
/// 修复前每个实例只在打开时读一次基线，之后一直用内存里的那份：两个实例各推一遍同样的本机变化，
/// 对方推的又经收件箱回到本机、在下一轮当成本机变化再推出去，计数每轮成倍涨。
#[test]
fn two_syncs_sharing_one_state_dir_count_each_choice_once() {
    let server = FakeServer::default();
    let phone = Device::new("two-syncs");
    phone.choose("问题");
    let mut first = phone.sync(&server);
    let mut second = phone.sync(&server);
    for _ in 0..12 {
        phone.choose("问题");
        first.cycle().unwrap();
        phone.merge_inbox();
        second.cycle().unwrap();
        phone.merge_inbox();
    }
    assert_eq!(phone.count("问题"), 13);
    assert_eq!(server.count("user", "问题"), 13);
}

/// 另一个实例正在一轮里（拿着锁）：这一轮不推不拉，放锁后照常。
#[test]
fn skips_the_round_while_another_instance_holds_the_lock() {
    let server = FakeServer::default();
    let phone = Device::new("locked");
    phone.choose("问题");
    let mut sync = phone.sync(&server);
    let held = std::fs::File::create(phone.state().join("learning.lock")).unwrap();
    held.lock().unwrap();
    let outcome = sync.cycle().unwrap();
    assert!(outcome.busy);
    assert_eq!(outcome.pushed, 0);
    assert_eq!(server.count("user", "问题"), 0);
    drop(held);
    assert_eq!(sync.cycle().unwrap().pushed, 1);
    assert_eq!(server.count("user", "问题"), 1);
}

/// 服务器上已经有出错累加出来的计数：本机读文件不认它，下一轮把差额作为负增量推回去，服务器上的坏数清掉。
#[test]
fn oversized_counts_on_the_server_are_cleared_by_the_next_round() {
    let server = FakeServer::default();
    let poisoned = LearningPush {
        counts: vec![qingjian_cloud_proto::CountDelta {
            table: "user".to_owned(),
            key: "问题".to_owned(),
            delta: 246_018_884,
            value: None,
        }],
        ..LearningPush::default()
    };
    server.push_learning(&poisoned).unwrap();
    let mac = Device::new("oversized");
    mac.choose("开发");
    let mut sync = mac.sync(&server);
    sync.cycle().unwrap();
    mac.merge_inbox();
    sync.cycle().unwrap();
    assert_eq!(server.count("user", "问题"), 0);
    assert_eq!(server.count("user", "开发"), 1);
}
