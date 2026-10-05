//! 会话里与青简 Cloud 有关的部分：大模型联想的结果（插在首选之后，不抢空格要上屏的那个；预测器按 `[predict]` 在 `config.rs` 里建）、
//! 润色、学习数据同步（`DataSync` 与 Mac 端同一套基线 + 增量 + 收件箱，收件箱在这里合并）。

use std::io::ErrorKind;

use qingjian_cloud_client::{Client, DataSync, DataSyncConfig, INBOX};
use qingjian_core::{Candidate, SurroundingText};

use super::Session;
use crate::clipboard::{ClipOffer, Clipboard};
use crate::cloud_config::CloudConfig;
use crate::entry::Entry;
use crate::rewrite::{Rewriter, Skill};

/// 云端的词与整句插在第几格起：第 0 格留给本地首选。
const CLOUD_POSITION: usize = 1;

impl Session {
    pub(super) fn connect(&mut self, cloud: &CloudConfig) {
        // 没有技能包就整个不建：改写按钮跟着不出现（打包漏了，`open` 里已经记过一条 error）
        if cloud.llm && !self.skills.is_empty() {
            self.rewriter = Some(Rewriter::new(
                Client::new(&cloud.server, &cloud.token),
                self.skills.clone(),
            ));
        }
        if let (true, Some(user_dir)) = (cloud.clipboard, &self.user_dir) {
            let client = Client::new(&cloud.server, &cloud.token);
            self.clipboard = Some(Clipboard::new(
                client,
                user_dir.join("cloud/clipboard.json"),
            ));
        }
        // 学习数据与设置跟 `sync` 走，输入日志跟 `logs` 走：服务器上是两个独立的开关
        if let (true, Some(user_dir)) = (cloud.sync || cloud.logs, &self.user_dir) {
            let started = DataSync::start(DataSyncConfig {
                server: cloud.server.clone(),
                token: cloud.token.clone(),
                ime_dir: user_dir.clone(),
                state_dir: user_dir.join("cloud"),
                sync_learning: cloud.sync,
                // config.toml 与 Mac 同步（模糊音、短语、词库开关……）；输入日志上传给纠错闭环，别的设备的不下载
                sync_logs: cloud.logs,
                log_download_dir: None,
                sync_config: cloud.sync,
            });
            match started {
                Ok(sync) => self.data_sync = Some(sync),
                Err(error) => tracing::warn!(%error, "学习数据同步启动失败"),
            }
        }
    }

    pub fn cloud_enabled(&self) -> bool {
        self.rewriter.is_some() || self.data_sync.is_some() || self.clipboard.is_some()
    }

    /// 润色器；私密输入框里没有（光标前的文字不能发出去）。
    pub fn rewriter(&self) -> Option<&Rewriter> {
        self.rewriter.as_ref().filter(|_| !self.engine.is_private())
    }

    /// 随包的改写技能（C 接口 `qj_rewrite_skills` 用）。
    pub fn rewrite_skills(&self) -> &[Skill] {
        &self.skills
    }

    /// 焦点在验证码、密码、信用卡号这类输入框：不学习、不记日志、不发云端（引擎的私密输入），
    /// 剪贴板与润色也停。
    pub fn set_private(&mut self, private: bool) {
        self.engine.set_private(private);
        if private && let Some(rewriter) = &self.rewriter {
            rewriter.cancel();
        }
    }

    /// 键盘弹出时调：拉一次别的设备的剪贴板。
    pub fn refresh_clipboard(&self) {
        if let Some(clipboard) = &self.clipboard {
            clipboard.refresh();
        }
    }

    pub fn clip_offer(&self) -> Option<ClipOffer> {
        if self.engine.is_private() {
            return None;
        }
        self.clipboard.as_ref().and_then(Clipboard::offer)
    }

    pub fn clip_handled(&self) {
        if let Some(clipboard) = &self.clipboard {
            clipboard.handled();
        }
    }

    pub fn clip_push(&self, text: &str) {
        if self.engine.is_private() {
            return;
        }
        if let Some(clipboard) = &self.clipboard {
            clipboard.push(text);
        }
    }

    pub fn clipboard_enabled(&self) -> bool {
        self.clipboard.is_some()
    }

    /// 宿主光标前后的文字。`before` 末尾若是我们写进去的 marked text（拼音），去掉再存。
    pub fn set_context(&mut self, before: &str, after: &str) {
        let before = before.strip_suffix(self.preedit.as_str()).unwrap_or(before);
        self.context = Some(SurroundingText {
            before: before.to_owned(),
            after: after.to_owned(),
        });
    }

    pub fn sync_now(&self) {
        if let Some(sync) = &self.data_sync {
            sync.sync_now();
        }
    }

    /// 键盘可见期间定时调：合并收件箱、按修改时间重载记忆、取回大模型结果。候选栏变了返回 true。
    pub fn poll(&mut self) -> bool {
        self.apply_inbox();
        let rescoped = self.poll_memory();
        // 设置变了（主 App 改的或从 Mac 同步来的）、App 删了当前对象（换了叠加层），候选要重排
        let config_changed = self.reload_config();
        let reloaded = (config_changed || rescoped) && self.composing();
        let Some(prediction) = self.engine.poll_prediction() else {
            return reloaded;
        };
        if !self.composing() || self.entries.is_empty() {
            return reloaded;
        }
        let mut position = CLOUD_POSITION.min(self.entries.len());
        let mut inserted = false;
        let mut insert = |entries: &mut Vec<Entry>, entry: Entry| {
            if entries.iter().any(|e| e.text() == entry.text()) {
                return;
            }
            entries.insert(position, entry);
            position += 1;
            inserted = true;
        };
        // 整句补全在配置里已关（`sentence: false`）；万一服务器还是给了，手机上也不要：候选栏放不下，用户要的是词
        let _ = prediction.sentence;
        for word in prediction.words {
            let candidate: Candidate = word.into_candidate();
            insert(&mut self.entries, Entry::Cloud(candidate));
        }
        inserted || reloaded
    }

    pub(super) fn request_prediction(&mut self) {
        let local: Vec<Candidate> = self
            .entries
            .iter()
            .filter_map(|entry| match entry {
                Entry::Local(candidate) => Some(candidate.clone()),
                _ => None,
            })
            .collect();
        if self
            .engine
            .request_prediction(self.context.clone(), &local)
            .is_none()
        {
            self.engine.cancel_prediction();
        }
    }

    /// 别的设备的学习增量：先合并、落盘，最后才删文件，中途被杀下次重来（最坏多算一次，不会丢）。
    pub(super) fn apply_inbox(&mut self) {
        let Some(path) = self.user_dir.as_ref().map(|dir| dir.join(INBOX)) else {
            return;
        };
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == ErrorKind::NotFound => return,
            Err(error) => {
                tracing::warn!(%error, "收件箱读不了");
                return;
            }
        };
        let applied = self.engine.learner_mut().merge_remote(&text);
        self.engine.flush_learning();
        if let Err(error) = std::fs::remove_file(&path) {
            tracing::warn!(%error, "收件箱删不掉");
        }
        tracing::info!(applied, "合并了别的设备的学习数据");
    }
}
