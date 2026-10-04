//! 一次键盘会话：持有 Engine，每次缓冲变化后重查一遍候选缓存起来，给 C ABI 按下标取。
//! 配了青简 Cloud 时还挂着大模型联想、润色与学习数据同步（见 `cloud.rs`）。

mod cloud;
mod config;

use std::path::{Path, PathBuf};

use qingjian_cloud_client::DataSync;
use qingjian_core::{Engine, SurroundingText};
use qingjian_dictionary::Dictionary;
use qingjian_learning::{FrequencyLearner, InputLog};
use qingjian_lm::BigramModel;

use crate::clipboard::Clipboard;
use crate::cloud_config::CloudConfig;
use crate::entry::Entry;
use crate::error::BridgeError;
use crate::rewrite::Rewriter;

/// 候选栏是横向滚动的一行，再多也翻不到，截断省得每键复制几百个候选。
const MAX_CANDIDATES: usize = 120;

pub struct Session {
    engine: Engine,

    /// 候选栏里的格子，下标与 Swift 那边显示的一致；云端结果回来后插在首选之后。
    entries: Vec<Entry>,

    /// 拼音行：Core 切好音节、补了 `'` 的显示串；没在组句时为空。
    preedit: String,

    /// 学习数据目录；同步的收件箱也在这下面。
    user_dir: Option<PathBuf>,

    /// 宿主光标前后的文字，发联想请求时带上。
    context: Option<SurroundingText>,

    data_sync: Option<DataSync>,

    rewriter: Option<Rewriter>,

    clipboard: Option<Clipboard>,

    /// 与 Mac 同格式的 `config.toml`（模糊音、双拼、繁体、领域词库、自定义短语等）与它上次套用时的修改时间。
    config_path: Option<PathBuf>,

    config_modified: Option<std::time::SystemTime>,

    /// 随包领域词库所在目录（`Data/dicts`）。
    dicts_dir: PathBuf,

    /// 青简 Cloud 的连接配置；云联想选青简 Cloud 时端点从这里来。离线为 `None`。
    cloud: Option<CloudConfig>,
}

impl Session {
    /// `data_dir` 里要有 `dict.qj`，`lm.qj` 可选（没有就退回词频整句）；
    /// `user_dir` 给了就从 `user.tsv` 读学习数据并在 [`Self::flush`] 时写回，没给只在内存里学；
    /// `config` 是设置文件 `config.toml`，不给就用 `user_dir` 下的（iOS 上没有完全访问时学习数据在扩展容器、设置在 App Group，两处分开）；
    /// `cloud` 给了就接上大模型与同步，没给完全离线。
    pub fn open(
        data_dir: &Path,
        user_dir: Option<&Path>,
        config: Option<&Path>,
        cloud: Option<CloudConfig>,
    ) -> Result<Self, BridgeError> {
        let dictionary = Dictionary::from_path(data_dir.join("dict.qj"))?;
        let learner = user_dir.map_or_else(FrequencyLearner::default, load_learner);
        let mut engine = Engine::new(dictionary).with_learner(Box::new(learner));
        let lm = data_dir.join("lm.qj");
        if lm.is_file() {
            match BigramModel::from_path(&lm) {
                Ok(model) => engine = engine.with_language_model(Box::new(model)),
                Err(error) => tracing::warn!(%error, "语言模型加载失败，使用词频整句"),
            }
        }
        // 只在连了青简 Cloud 且要上传时记日志：离线用户的输入不落任何日志
        if let (Some(dir), Some(cloud)) = (user_dir, &cloud)
            && cloud.logs
            && cloud.sync
        {
            engine =
                engine.with_input_logger(Box::new(InputLog::open(dir.join("input-log.jsonl"))));
        }
        let mut session = Self {
            engine,
            entries: Vec::new(),
            preedit: String::new(),
            user_dir: user_dir.map(Path::to_path_buf),
            context: None,
            data_sync: None,
            rewriter: None,
            clipboard: None,
            config_path: config
                .map(Path::to_path_buf)
                .or_else(|| user_dir.map(|dir| dir.join("config.toml"))),
            config_modified: None,
            dicts_dir: data_dir.join("dicts"),
            cloud: cloud.clone(),
        };
        session.reload_config();
        if let Some(cloud) = cloud {
            session.connect(&cloud);
        }
        session.apply_inbox();
        Ok(session)
    }

    pub fn composing(&self) -> bool {
        !self.engine.composition().is_empty()
    }

    pub fn preedit(&self) -> &str {
        &self.preedit
    }

    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    pub fn push(&mut self, c: char) {
        self.engine.push(c);
        self.refresh();
    }

    pub fn backspace(&mut self) {
        self.engine.backspace();
        self.refresh();
    }

    pub fn clear(&mut self) {
        self.engine.clear();
        self.refresh();
    }

    /// 上屏第 `index` 格；本地候选只吃掉一部分拼音时剩下的留在缓冲区，接着出候选。
    pub fn commit(&mut self, index: usize) -> Option<String> {
        let text = match self.entries.get(index)? {
            Entry::Local(candidate) | Entry::Cloud(candidate) => {
                let candidate = candidate.clone();
                self.engine.commit(&candidate)
            }
            Entry::Sentence(sentence) => {
                let sentence = sentence.clone();
                self.engine.accept_prediction(&sentence)
            }
        };
        self.refresh();
        Some(text)
    }

    /// 敲过的字母原样上屏（回车）。
    pub fn take_raw(&mut self) -> String {
        let text = self.engine.take_raw();
        self.refresh();
        text
    }

    /// 没在组句时的标点：中文模式转全角，不需要转的原样返回并记成直通字符。
    pub fn punctuate(&mut self, c: char) -> String {
        match self.engine.punctuate(c) {
            Some(text) => text.to_owned(),
            None => {
                self.engine.note_passthrough(c);
                c.to_string()
            }
        }
    }

    /// 没在组字时直接输出的字符（空格、回车）告诉引擎，输入日志里的句子边界才对。
    pub fn note_passthrough(&mut self, c: char) {
        self.engine.note_passthrough(c);
    }

    /// 键盘收起或进入后台时调，学习数据落盘（键盘扩展随时可能被系统杀掉），再催一轮同步。
    pub fn flush(&mut self) {
        self.engine.flush_learning();
        self.sync_now();
    }

    fn refresh(&mut self) {
        self.entries.clear();
        self.preedit.clear();
        if !self.composing() {
            self.engine.cancel_prediction();
            return;
        }
        match self.engine.query() {
            Ok(query) => {
                self.preedit = query.marked_text();
                self.entries.extend(
                    query
                        .candidates
                        .items
                        .into_iter()
                        .take(MAX_CANDIDATES)
                        .map(Entry::Local),
                );
            }
            // 拼不成音节（如 `vvv`）：显示原样输入，没有候选，回车原样上屏。
            Err(_) => self.preedit = self.engine.composition().text().to_owned(),
        }
        self.request_prediction();
    }
}

/// 读不了就退回只在内存里学，不拿空表覆盖用户文件。
fn load_learner(dir: &Path) -> FrequencyLearner {
    let path: PathBuf = dir.join("user.tsv");
    FrequencyLearner::from_path(&path).unwrap_or_else(|error| {
        tracing::error!(path = %path.display(), %error, "学习数据读取失败，本次只在内存里学习");
        FrequencyLearner::default()
    })
}
