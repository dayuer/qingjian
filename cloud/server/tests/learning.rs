//! 学习数据同步端到端：两台「Mac」各有一份上游的 FrequencyLearner（即打了补丁的输入法），
//! 经服务器交换增量，验证合并结果正确、不重复计数、本机同时在学的不被覆盖。

mod common;

use std::path::Path;

use qingjian_cloud_client::{Client, INBOX, LearningSync};
use qingjian_core::sentence::Context;
use qingjian_core::{Candidate, CandidateKind, Learner};
use qingjian_learning::FrequencyLearner;

use common::TestServer;

/// 一台设备：输入法的数据目录 + 内存里的学习器 + Cloud 的同步。
struct Device {
    _dir: tempfile::TempDir,
    ime_dir: std::path::PathBuf,
    learner: FrequencyLearner,
    sync: LearningSync,
}

impl Device {
    fn new(server: &TestServer, name: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let ime_dir = dir.path().join("Qingjian");
        std::fs::create_dir_all(&ime_dir).unwrap();
        let client = Client::new(&server.url, &server.store.add_device(name).unwrap());
        let sync = LearningSync::open(client, &ime_dir, &dir.path().join("cloud")).unwrap();
        Self {
            learner: FrequencyLearner::from_path(ime_dir.join("user.tsv")).unwrap(),
            ime_dir,
            sync,
            _dir: dir,
        }
    }

    fn pick(&mut self, text: &str, times: usize) {
        for _ in 0..times {
            self.learner.record(&candidate(text));
        }
    }

    /// 输入法每秒一拍做的事：落盘、看收件箱（与 apps/macos 的 host/cloud/inbox.rs 一致）。
    fn ime_tick(&mut self) {
        self.learner.flush();
        let path = self.ime_dir.join(INBOX);
        if let Ok(text) = std::fs::read_to_string(&path) {
            self.learner.merge_remote(&text);
            std::fs::remove_file(path).unwrap();
        }
    }

    /// 同步一轮并让输入法合并，再同步一轮把合并确认进基线。
    fn sync(&mut self) {
        self.ime_tick();
        self.sync.cycle().unwrap();
        self.ime_tick();
        self.sync.cycle().unwrap();
    }
}

fn candidate(text: &str) -> Candidate {
    Candidate {
        text: text.to_owned(),
        kind: CandidateKind::Chinese,
        syllables: Vec::new(),
        reading: None,
        translation: None,
        aux_code: None,
    }
}

fn file(dir: &Path, name: &str) -> String {
    std::fs::read_to_string(dir.join(name)).unwrap_or_default()
}

#[test]
fn counts_from_both_devices_add_up_without_double_counting() {
    let server = TestServer::start();
    let mut home = Device::new(&server, "home");
    let mut office = Device::new(&server, "office");

    home.pick("青简", 3);
    office.pick("青简", 2);
    office.pick("同步", 1);
    home.sync();
    office.sync();
    home.sync();
    assert_eq!(home.learner.weight("青简"), 5);
    assert_eq!(office.learner.weight("青简"), 5);
    assert_eq!(home.learner.weight("同步"), 1);

    // 再同步几轮什么都不该变
    for _ in 0..3 {
        home.sync();
        office.sync();
    }
    assert_eq!(home.learner.weight("青简"), 5);
    assert_eq!(office.learner.weight("青简"), 5);
}

#[test]
fn local_learning_during_pending_inbox_is_kept() {
    let server = TestServer::start();
    let mut home = Device::new(&server, "home");
    let mut office = Device::new(&server, "office");
    home.pick("开发", 4);
    home.sync();

    // office 拉到收件箱，但输入法还没合并时用户又打了两次
    office.ime_tick();
    let outcome = office.sync.cycle().unwrap();
    assert!(outcome.waiting);
    office.pick("开发", 2);
    office.ime_tick();
    office.sync();
    home.sync();
    assert_eq!(office.learner.weight("开发"), 6);
    assert_eq!(home.learner.weight("开发"), 6);
}

#[test]
fn all_tables_and_deletions_propagate() {
    let server = TestServer::start();
    let mut home = Device::new(&server, "home");
    let mut office = Device::new(&server, "office");

    home.learner.record_choice("kf", "开发");
    home.learner.record_typo("hs", "shi");
    home.learner.learn_english("GitHub");
    home.learner
        .learn_word("青简", &["qing".to_owned(), "jian".to_owned()]);
    home.learner
        .learn_word("云端", &["yun".to_owned(), "duan".to_owned()]);
    home.learner
        .record_transition(Context::after_two("我们", "在"), "开发", 2);
    home.sync();
    office.sync();
    assert_eq!(office.learner.choice_weight("kf", "开发"), 1);
    assert_eq!(office.learner.typo_count("hs", "shi"), 1);
    assert_eq!(office.learner.english_count(), 1);
    assert!(file(&office.ime_dir, "user-english.tsv").contains("GitHub\t1"));
    assert_eq!(office.learner.word_count(), 2);
    assert_eq!(
        office.learner.user_ngram().unwrap().to_tsv(),
        home.learner.user_ngram().unwrap().to_tsv()
    );

    // office 删掉一个用户词（候选上按删除键），home 也跟着删
    office.learner.forget("云端");
    office.sync();
    home.sync();
    assert_eq!(home.learner.word_count(), 1);
    assert_eq!(office.learner.word_count(), 1);
}

#[test]
fn interrupted_push_is_not_counted_twice() {
    let server = TestServer::start();
    let addr = server.addr();
    let mut home = Device::new(&server, "home");
    home.pick("断网", 3);
    home.ime_tick();
    let store = server.stop();
    assert!(home.sync.cycle().is_err());
    let server = TestServer::start_on(addr, store);
    home.sync();
    let mut office = Device::new(&server, "office");
    office.sync();
    assert_eq!(office.learner.weight("断网"), 3);
}

mod config {
    use std::path::Path;

    use qingjian_cloud_client::{Client, ConfigOutcome, ConfigSync};

    use super::common::TestServer;

    fn device(server: &TestServer, name: &str) -> (tempfile::TempDir, ConfigSync) {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Qingjian")).unwrap();
        let client = Client::new(&server.url, &server.store.add_device(name).unwrap());
        let sync = ConfigSync::open(
            client,
            &dir.path().join("Qingjian"),
            &dir.path().join("cloud"),
        )
        .unwrap();
        (dir, sync)
    }

    fn config(dir: &Path) -> String {
        std::fs::read_to_string(dir.join("Qingjian/config.toml")).unwrap_or_default()
    }

    fn write(dir: &Path, text: &str) {
        std::fs::write(dir.join("Qingjian/config.toml"), text).unwrap();
    }

    #[test]
    fn edits_flow_both_ways_and_conflicts_keep_a_backup() {
        let server = TestServer::start();
        let (home_dir, mut home) = device(&server, "home");
        let (office_dir, mut office) = device(&server, "office");

        write(home_dir.path(), "[general]\nshuangpin = \"xiaohe\"\n");
        assert_eq!(home.cycle().unwrap(), ConfigOutcome::Uploaded);
        assert_eq!(office.cycle().unwrap(), ConfigOutcome::Downloaded);
        assert_eq!(
            config(office_dir.path()),
            "[general]\nshuangpin = \"xiaohe\"\n"
        );
        assert_eq!(office.cycle().unwrap(), ConfigOutcome::Unchanged);

        write(
            office_dir.path(),
            "[[custom_phrases]]\ninput = \"dz\"\ntext = \"地址\"\n",
        );
        assert_eq!(office.cycle().unwrap(), ConfigOutcome::Uploaded);
        assert_eq!(home.cycle().unwrap(), ConfigOutcome::Downloaded);
        assert!(config(home_dir.path()).contains("地址"));

        // 两边都改：较新的赢，另一份进备份目录
        write(home_dir.path(), "# home\n");
        write(office_dir.path(), "# office\n");
        assert_eq!(office.cycle().unwrap(), ConfigOutcome::Uploaded);
        assert_eq!(home.cycle().unwrap(), ConfigOutcome::Conflict);
        let backups = std::fs::read_dir(home_dir.path().join("Qingjian/sync/config-conflicts"))
            .unwrap()
            .count();
        assert_eq!(backups, 1);
        home.cycle().unwrap();
        office.cycle().unwrap();
        assert_eq!(config(home_dir.path()), config(office_dir.path()));
    }
}
