//! 一次键盘会话：持有 Engine，每次缓冲变化后重查一遍候选缓存起来，给 C ABI 按下标取。
//! 配了青简 Cloud 时还挂着大模型联想、润色与学习数据同步（见 `cloud.rs`）；
//! 有学习数据目录时挂着本地记忆：对象叠加层与打字提示（见 `memory/`）。

mod cloud;
mod config;
mod memory;
mod model;

use std::path::{Path, PathBuf};

use qingjian_cloud_client::DataSync;
use qingjian_core::{Engine, Learner, SurroundingText};
use qingjian_dictionary::{Dictionary, WordList};
use qingjian_learning::{FrequencyLearner, InputLog};
use qingjian_lm::BigramModel;

use self::memory::LiveMemory;
use self::model::ModelState;

pub use self::model::{MODEL_ACTIVE, MODEL_FAILED, MODEL_IDLE, MODEL_LOADING};

pub use self::memory::DroppedNotes;
use crate::clipboard::Clipboard;
use crate::cloud_config::CloudConfig;
use crate::entry::Entry;
use crate::error::BridgeError;
use crate::rewrite::{Rewriter, Skill};

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

    /// 随包的改写技能；`data_dir/skills` 里读一次。空的话改写整个不出现。
    skills: Vec<Skill>,

    clipboard: Option<Clipboard>,

    /// 与 Mac 同格式的 `config.toml`（模糊音、双拼、繁体、领域词库、自定义短语等）与它上次套用时的修改时间。
    config_path: Option<PathBuf>,

    config_modified: Option<std::time::SystemTime>,

    /// 随包领域词库所在目录（`Data/dicts`）。
    dicts_dir: PathBuf,

    /// 青简 Cloud 的连接配置；云联想选青简 Cloud 时端点从这里来。离线为 `None`。
    cloud: Option<CloudConfig>,

    /// 本地记忆；没有学习数据目录（只在内存里学）时为 `None`，学习器也就不分区。
    memory: Option<LiveMemory>,

    /// 本地神经整句模型的加载状态（见 `model` 模块）。
    model: ModelState,

    /// 随包英文词表路径；`english_loaded` 为假且见到像英文的输入时读进来挂上引擎。
    english_pending: Option<PathBuf>,

    /// 英文词表已挂上引擎（内存吃紧先卸它：比模型小，触发器还会再加载）。
    english_loaded: bool,
}

impl Session {
    /// `data_dir` 里要有 `dict.qj`，`lm.qj` 可选（没有就退回词频整句）；
    /// `user_dir` 给了就从 `user.tsv` 读学习数据并在 [`Self::flush`] 时写回，记忆在它下面的 `memory/`；没给只在内存里学、没有记忆；
    /// `config` 是设置文件 `config.toml`，不给就用 `user_dir` 下的（iOS 上没有完全访问时学习数据在扩展容器、设置在 App Group，两处分开）；
    /// `cloud` 给了就接上大模型与同步，没给完全离线。
    pub fn open(
        data_dir: &Path,
        user_dir: Option<&Path>,
        config: Option<&Path>,
        cloud: Option<CloudConfig>,
    ) -> Result<Self, BridgeError> {
        let dictionary = Dictionary::from_path(data_dir.join("dict.qj"))?;
        let (learner, mut memory): (Box<dyn Learner>, Option<LiveMemory>) = match user_dir {
            Some(dir) => {
                let (learner, memory) = LiveMemory::open(dir);
                (Box::new(learner), Some(memory))
            }
            None => (Box::new(FrequencyLearner::default()), None),
        };
        // 名单里还没有首字母的对象补上（通讯录按字母分组要用，见 memory::initial）。
        // 这是唯一还拿着词库的地方——再往下 dictionary 就被 move 进引擎了。
        if let Some(memory) = memory.as_mut() {
            memory.fill_contact_initials(|name| crate::memory::initial_of(&dictionary, name));
        }
        let mut engine = Engine::new(dictionary).with_learner(learner);
        let lm = data_dir.join("lm.qj");
        if lm.is_file() {
            match BigramModel::from_path(&lm) {
                Ok(model) => engine = engine.with_language_model(Box::new(model)),
                Err(error) => tracing::warn!(%error, "语言模型加载失败，使用词频整句"),
            }
        }
        // 只在登录了且开了「上传输入日志」时记日志：离线或没开的用户，输入不落任何日志
        if let (Some(dir), Some(cloud)) = (user_dir, &cloud)
            && cloud.logs
        {
            engine =
                engine.with_input_logger(Box::new(InputLog::open(dir.join("input-log.jsonl"))));
        }
        // 英文词表随包走（`Data/english.tsv`），但懒加载：解析后驻留约 13MB（2.2MB 的表）、峰值约 30MB，
        // 键盘扩展和 44MB 的通变模型塞不下，首次见到像英文的输入（见 `looks_english`）才读
        let english_pending = data_dir
            .join("english.tsv")
            .is_file()
            .then(|| data_dir.join("english.tsv"));
        // 技能包随包走（`Data/skills`），会话打开时读一次；打包漏了它改写就整个用不了，这里记一条显眼的
        let skills = crate::rewrite::load_skills(&data_dir.join("skills"));
        if skills.is_empty() && cloud.as_ref().is_some_and(|cloud| cloud.llm) {
            tracing::error!("没有技能包，改写用不了（assets/skills 没打进包？）");
        }
        let mut session = Self {
            engine,
            entries: Vec::new(),
            preedit: String::new(),
            user_dir: user_dir.map(Path::to_path_buf),
            context: None,
            data_sync: None,
            rewriter: None,
            skills,
            clipboard: None,
            config_path: config
                .map(Path::to_path_buf)
                .or_else(|| user_dir.map(|dir| dir.join("config.toml"))),
            config_modified: None,
            dicts_dir: data_dir.join("dicts"),
            cloud: cloud.clone(),
            memory,
            model: ModelState::default(),
            english_pending,
            english_loaded: false,
        };
        session.reload_config();
        if let Some(cloud) = cloud {
            session.connect(&cloud);
        }
        session.apply_inbox();
        session.rebuild_hints();
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

    /// 开始异步加载本地神经整句模型（含章·通变）；已在加载或在用返回 `false`。
    pub fn load_model(&mut self, path: &Path, p2c: bool) -> bool {
        self.model.load(path, p2c)
    }

    /// 模型状态（[`MODEL_IDLE`] 等四个取值）；顺带取加载线程的结果。
    pub fn model_state(&mut self) -> u8 {
        self.attach_loaded_model();
        self.model.state()
    }

    /// 卸载本地模型：重打分停用，状态回未加载。内存吃紧时腾地方。
    pub fn unload_model(&mut self) {
        self.engine.set_async_sentence_scorer(None);
        self.model.unload();
    }

    /// 加载线程出了结果就接上引擎；接上后下一次查询起整句带重打分。
    pub(crate) fn attach_loaded_model(&mut self) {
        if let Some(scorer) = self.model.poll() {
            self.engine.set_async_sentence_scorer(Some(scorer));
        }
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
        self.note_committed(&text);
        self.refresh();
        Some(text)
    }

    /// 敲过的字母原样上屏（回车）。
    pub fn take_raw(&mut self) -> String {
        let text = self.engine.take_raw();
        self.note_committed(&text);
        self.refresh();
        text
    }

    /// 没在组句时的标点：中文模式转全角，不需要转的原样返回并记成直通字符。
    pub fn punctuate(&mut self, c: char) -> String {
        let text = match self.engine.punctuate(c) {
            Some(text) => text.to_owned(),
            None => {
                self.engine.note_passthrough(c);
                c.to_string()
            }
        };
        self.note_committed(&text);
        self.update_hint();
        text
    }

    /// 没在组字时直接输出的字符（空格、回车）告诉引擎，输入日志里的句子边界才对。
    pub fn note_passthrough(&mut self, c: char) {
        self.engine.note_passthrough(c);
        self.note_committed(c.encode_utf8(&mut [0; 4]));
        self.update_hint();
    }

    /// 键盘收起或进入后台时调，学习数据落盘（键盘扩展随时可能被系统杀掉），清掉最近上屏的字，再催一轮同步。
    pub fn flush(&mut self) {
        self.retry_pending();
        self.engine.flush_learning();
        self.reset_context();
        self.sync_now();
    }

    /// 每次按键后：先补写拿不到锁时留下的待办，再重查候选。
    fn refresh(&mut self) {
        self.retry_pending();
        self.load_english_if_english_like();
        self.refresh_candidates();
    }

    /// 见到像英文的输入才加载英文词表（驻留约 13MB，纯拼音用户整场不付这笔账）。
    /// 加载失败也清掉待办并记日志：反复重试只会每个键都卡一次读盘。
    fn load_english_if_english_like(&mut self) {
        if self.english_loaded {
            return;
        }
        let Some(path) = self.english_pending.clone() else {
            return;
        };
        let typed = self.engine.composition().text();
        if !looks_english(typed) {
            return;
        }
        match WordList::from_path(&path) {
            Ok(words) => {
                self.engine.set_english(Some(words));
                self.english_loaded = true;
            }
            Err(error) => {
                // 失败也清掉待办：反复重试只会每个键都卡一次读盘
                self.english_pending = None;
                tracing::warn!(%error, "英文词表加载失败，中英混输没有英文候选");
            }
        }
    }

    /// 内存吃紧先卸英文表（约 13MB，比模型小、触发器下次还会再加载）。
    pub fn unload_english(&mut self) {
        if self.english_loaded {
            self.engine.set_english(None);
            self.english_loaded = false;
        }
    }

    fn refresh_candidates(&mut self) {
        self.entries.clear();
        self.preedit.clear();
        if self.composing() {
            // 宿主前文给词级排序（Core 的 query/left_context.rs）；`clear()` 会清掉，每键设一次；私密输入不给
            let before = (!self.engine.is_private())
                .then(|| self.context.as_ref().map(|c| c.before.clone()))
                .flatten();
            self.engine.set_rescoring_context(before);
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
        } else {
            self.engine.cancel_prediction();
        }
        self.update_hint();
    }
}

/// 像英文的输入：出现拼音里不可能的辅音相邻。拼音里两个辅音相邻只有三种情况——
/// 前一个是韵尾 `n` / `g` / `r`（er），前一个是 z / c / s 后跟 `h`（zh ch sh 声母），
/// 或 `v` 跟在 l / n 后（ü）。都不是的相邻（android 的 `dr`、hello 的 `ll`、rust 的 `st`）就是英文。
fn looks_english(buffer: &str) -> bool {
    let letters: Vec<char> = buffer
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .map(|c| c.to_ascii_lowercase())
        .collect();
    letters.windows(2).any(|pair| {
        let (a, b) = (pair[0], pair[1]);
        // y / w 当元音（半元音声母，ye / wa 都是合法音节起头，不构成辅音丛的信号）
        let consonant = |c: char| !"aeiouyw".contains(c);
        consonant(a)
            && consonant(b)
            && !"ngr".contains(a)
            && !("zcs".contains(a) && b == 'h')
            // v 只出现在 lv / nv 里，后面接什么都可能是下一音节（lvy e、nv hai）
            && a != 'v'
            && !(b == 'v' && (a == 'l' || a == 'n'))
    })
}

#[cfg(test)]
mod tests {
    use super::looks_english;

    #[test]
    fn english_like_inputs() {
        // 拼音里不可能的辅音相邻才算像英文：android 的 ndr、hello 的 ll、rust 的 st
        assert!(looks_english("android"));
        assert!(looks_english("andro"));
        assert!(looks_english("hello"));
        assert!(looks_english("rust"));
        // 纯拼音不触发：zhuangzhuang 的 ng、anr 的 nr 都是合法相邻
        assert!(!looks_english("nihao"));
        assert!(!looks_english("zhuangzhuang"));
        assert!(!looks_english("anr"));
        // v 在拼音里只跟 l / n（ü），lv nv 不算英文
        assert!(!looks_english("lvye"));
        assert!(!looks_english("nvhai"));
    }
}
