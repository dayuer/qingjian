//! 分区学习器：全局层（学习数据目录的 `user.tsv`）之外，选了对象时叠对象层。
//! **按人隔离**：选了人时写只进这个人的对象层（不碰全局，也不记个人 n-gram——那个只读全局、写不进去）；
//! 没选人（「不指定」）时写全局。读一律是全局 + 对象层加权求和。
//! 用户词、个人 n-gram、英文词表要返回引用，没法现场叠加，一律读全局。删词连当前打开的叠加层一起删。
//! 包装层必须逐个转发 `Learner` 的全部方法，漏一个就会被 trait 的缺省实现悄悄吞掉。私密输入由外面的 `MutedLearner` 挡写。

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use qingjian_core::sentence::{Context, UserNgram};
use qingjian_core::{Candidate, Forgotten, Learner};
use qingjian_dictionary::{Dictionary, WordList};
use qingjian_learning::FrequencyLearner;

use super::{Overlay, ScopeHandle, load_layer, lock};

pub struct ScopedLearner {
    global: FrequencyLearner,

    overlay: Arc<Mutex<Overlay>>,

    /// 叠加层的倍数：产品里是 [`Self::OVERLAY_WEIGHT`]，回放调参时经 [`Self::open_with_weight`] 换。
    weight: u32,

    memory_dir: PathBuf,
}

impl ScopedLearner {
    /// 叠加层计数的倍数（spec 定 4，`examples/overlay_replay.rs` 回放调）。
    pub const OVERLAY_WEIGHT: u32 = 4;

    pub fn open(user_dir: &Path, memory_dir: &Path, contact: Option<&str>) -> Self {
        Self::open_with_weight(user_dir, memory_dir, contact, Self::OVERLAY_WEIGHT)
    }

    /// 同 [`Self::open`]，叠加倍数自定。只给回放调参用，不进产品配置。
    pub fn open_with_weight(
        user_dir: &Path,
        memory_dir: &Path,
        contact: Option<&str>,
        weight: u32,
    ) -> Self {
        Self {
            global: load_layer(user_dir),
            overlay: Arc::new(Mutex::new(Overlay::open(memory_dir, contact))),
            weight,
            memory_dir: memory_dir.to_path_buf(),
        }
    }

    /// 会话换场景、换对象用的把手。
    pub fn handle(&self) -> ScopeHandle {
        ScopeHandle::new(Arc::clone(&self.overlay), self.memory_dir.clone())
    }

    fn count(&self, read: impl Fn(&FrequencyLearner) -> u32) -> u32 {
        read(&self.global).saturating_add(lock(&self.overlay).count(self.weight, &read))
    }

    /// 选了人只写这个人的对象层（按人隔离）；没选人写全局。
    fn write(&mut self, mut f: impl FnMut(&mut FrequencyLearner)) {
        let mut overlay = lock(&self.overlay);
        if overlay.isolated() {
            overlay.write(&mut f);
        } else {
            f(&mut self.global);
        }
    }
}

impl Learner for ScopedLearner {
    fn record(&mut self, candidate: &Candidate) {
        self.write(|layer| layer.record(candidate));
    }

    fn weight(&self, text: &str) -> u32 {
        self.count(|layer| layer.weight(text))
    }

    fn record_choice(&mut self, input: &str, text: &str) {
        self.write(|layer| layer.record_choice(input, text));
    }

    fn choice_weight(&self, input: &str, text: &str) -> u32 {
        self.count(|layer| layer.choice_weight(input, text))
    }

    fn record_raw(&mut self, input: &str) {
        self.write(|layer| layer.record_raw(input));
    }

    fn raw_count(&self, input: &str) -> u32 {
        self.count(|layer| layer.raw_count(input))
    }

    fn unrecord(&mut self, text: &str) {
        self.write(|layer| layer.unrecord(text));
    }

    fn unrecord_choice(&mut self, input: &str, text: &str) {
        self.write(|layer| layer.unrecord_choice(input, text));
    }

    fn unrecord_transition(&mut self, context: Context<'_>, word: &str, times: u32) {
        // 个人 n-gram 只有全局那一份，选了人时不撤（那会儿也没记）
        if !lock(&self.overlay).isolated() {
            self.global.unrecord_transition(context, word, times);
        }
    }

    fn learn_word(&mut self, text: &str, syllables: &[String]) {
        self.global.learn_word(text, syllables);
    }

    fn user_words(&self) -> Option<&Dictionary> {
        self.global.user_words()
    }

    fn learn_english(&mut self, word: &str) {
        self.global.learn_english(word);
    }

    fn user_english(&self) -> Option<&WordList> {
        self.global.user_english()
    }

    fn record_transition(&mut self, context: Context<'_>, word: &str, times: u32) {
        // 按人隔离：选了人时不记个人 n-gram（它只读全局，记了会在别的对象下冒出来）
        if !lock(&self.overlay).isolated() {
            self.global.record_transition(context, word, times);
        }
    }

    fn user_ngram(&self) -> Option<&UserNgram> {
        self.global.user_ngram()
    }

    fn record_typo(&mut self, typed: &str, intended: &str) {
        self.write(|layer| layer.record_typo(typed, intended));
    }

    fn unrecord_typo(&mut self, typed: &str, intended: &str) {
        self.write(|layer| layer.unrecord_typo(typed, intended));
    }

    fn typo_count(&self, typed: &str, intended: &str) -> u32 {
        self.count(|layer| layer.typo_count(typed, intended))
    }

    fn forget(&mut self, text: &str) -> Forgotten {
        let mut forgotten = self.global.forget(text);
        lock(&self.overlay).write(|layer| {
            let more = layer.forget(text);
            forgotten.user_word |= more.user_word;
            forgotten.learning |= more.learning;
        });
        forgotten
    }

    fn forget_english(&mut self, word: &str) -> bool {
        let mut found = self.global.forget_english(word);
        lock(&self.overlay).write(|layer| found |= layer.forget_english(word));
        found
    }

    fn merge_remote(&mut self, inbox: &str) -> usize {
        self.global.merge_remote(inbox)
    }

    fn flush(&mut self) {
        self.global.flush();
        lock(&self.overlay).flush();
    }
}
