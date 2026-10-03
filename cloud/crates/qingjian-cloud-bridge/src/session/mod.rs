//! 一次键盘会话：持有 Engine，每次缓冲变化后重查一遍候选缓存起来，给 C ABI 按下标取。

use std::path::{Path, PathBuf};

use qingjian_core::{Candidate, Engine};
use qingjian_dictionary::Dictionary;
use qingjian_learning::FrequencyLearner;
use qingjian_lm::BigramModel;

use crate::error::BridgeError;

/// 候选栏是横向滚动的一行，再多也翻不到，截断省得每键复制几百个候选。
const MAX_CANDIDATES: usize = 120;

pub struct Session {
    engine: Engine,

    /// 上一次 `query` 的候选，下标与 Swift 那边显示的一致。
    candidates: Vec<Candidate>,

    /// 拼音行：Core 切好音节、补了 `'` 的显示串；没在组句时为空。
    preedit: String,
}

impl Session {
    /// `data_dir` 里要有 `dict.qj`，`lm.qj` 可选（没有就退回词频整句）；
    /// `user_dir` 给了就从 `user.tsv` 读学习数据并在 [`Self::flush`] 时写回，没给只在内存里学。
    pub fn open(data_dir: &Path, user_dir: Option<&Path>) -> Result<Self, BridgeError> {
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
        Ok(Self {
            engine,
            candidates: Vec::new(),
            preedit: String::new(),
        })
    }

    pub fn composing(&self) -> bool {
        !self.engine.composition().is_empty()
    }

    pub fn preedit(&self) -> &str {
        &self.preedit
    }

    pub fn candidates(&self) -> &[Candidate] {
        &self.candidates
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

    /// 上屏第 `index` 个候选；候选只吃掉一部分拼音时剩下的留在缓冲区，接着出候选。
    pub fn commit(&mut self, index: usize) -> Option<String> {
        let candidate = self.candidates.get(index)?.clone();
        let text = self.engine.commit(&candidate);
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

    /// 键盘收起或进入后台时调，学习数据落盘（键盘扩展随时可能被系统杀掉）。
    pub fn flush(&mut self) {
        self.engine.flush_learning();
    }

    fn refresh(&mut self) {
        self.candidates.clear();
        self.preedit.clear();
        if !self.composing() {
            return;
        }
        match self.engine.query() {
            Ok(query) => {
                self.preedit = query.marked_text();
                self.candidates
                    .extend(query.candidates.items.into_iter().take(MAX_CANDIDATES));
            }
            // 拼不成音节（如 `vvv`）：显示原样输入，没有候选，回车原样上屏。
            Err(_) => self.preedit = self.engine.composition().text().to_owned(),
        }
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
