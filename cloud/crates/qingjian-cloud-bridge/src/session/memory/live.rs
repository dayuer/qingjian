//! 会话里的本地记忆状态：当前场景与对象、换层把手、名单、当前对象的卡片与提示索引、最近上屏的字、两条提示。
//! 键盘只读记忆文件（「记一笔」「知道了」与当前场景除外），App 改了按修改时间重载（见 `Session::poll_memory`）。
//! 读不了（开机后还没解锁过、数据保护挡住）就留着内存里原来的，不当成空；
//! 写时拿不到文件锁（键盘只等 200 毫秒）的「记一笔」与切场景先放进内存待办，之后重试（见 `pending`）。

use std::collections::HashSet;
use std::path::Path;
use std::time::SystemTime;

use super::pending::PendingWrites;
use crate::memory::{
    Card, Contact, Hint, HintIndex, KEYBOARD_LOCK_TIMEOUT, LocalDate, MemoryStore, RecentText,
    sanitized_scope,
};
use crate::scope::{ScopeHandle, ScopeState, ScopedLearner};

pub(crate) struct LiveMemory {
    pub(super) store: MemoryStore,

    pub(super) handle: ScopeHandle,

    pub(super) state: ScopeState,

    pub(super) contacts: Vec<Contact>,

    /// 当前对象的卡片；没选对象时为空。
    pub(super) cards: Vec<Card>,

    pub(super) hints: HintIndex,

    pub(super) recent: RecentText,

    /// 切到对象时算出的日子提醒，优先于匹配提示；「知道了」后清掉。
    pub(super) today: Option<Hint>,

    /// 最近一次 refresh 匹配到的提示。
    pub(super) current: Option<Hint>,

    /// `contacts.json`、`state.json`、当前对象 `cards.json` 上次读时的修改时间。
    pub(super) stamp: [Option<SystemTime>; 3],

    /// 拿不到锁、等着重试的写入。
    pub(super) pending: PendingWrites,

    /// 换了对象但卡片还没读进来（当时拿不到锁或读不了）：先当没有卡，refresh 时补读。
    pub(super) cards_stale: bool,
}

impl LiveMemory {
    /// 读名单与 `state.json`（对象不在名单上就退回不指定），按当前场景开分区学习器，读入「知道了」的记录。
    /// 提示索引由会话随后建（要用引擎的语言模型切词）。
    pub(in crate::session) fn open(user_dir: &Path) -> (ScopedLearner, Self) {
        let store = MemoryStore::open_with_lock_timeout(user_dir, KEYBOARD_LOCK_TIMEOUT);
        let contacts = store.contacts();
        let state = sanitized_scope(store.state(), &contacts);
        // 键盘每次起来顺手清掉改名满 30 天的老场景学习目录（不在拿锁的路径上，见 MemoryStore）
        store.sweep_migrated_dirs(LocalDate::today());
        let learner = ScopedLearner::open(user_dir, store.root(), state.contact_id.as_deref());
        let handle = learner.handle();
        let cards = state
            .contact_id
            .as_deref()
            .map(|id| store.cards(id))
            .unwrap_or_default();
        let known: HashSet<String> = store
            .snapshot()
            .map(|snapshot| {
                snapshot
                    .cards
                    .into_values()
                    .flatten()
                    .map(|card| card.id)
                    .collect()
            })
            .unwrap_or_default();
        let mut hints = HintIndex::default();
        hints.set_dismissed(store.dismissed(LocalDate::today(), &known));
        let stamp = store.stamp(state.contact_id.as_deref());
        let root = store.root().to_path_buf();
        let memory = Self {
            store,
            handle,
            state,
            contacts,
            cards,
            hints,
            recent: RecentText::default(),
            today: None,
            current: None,
            stamp,
            pending: PendingWrites::open(&root),
            cards_stale: false,
        };
        (learner, memory)
    }

    /// 选了对象才有提示与日子提醒（私密输入由会话另挡）。
    pub(super) fn shows_hints(&self) -> bool {
        self.contact().is_some()
    }

    pub(super) fn contact(&self) -> Option<&Contact> {
        let id = self.state.contact_id.as_deref()?;
        self.contacts.iter().find(|c| c.id == id)
    }

    /// 重读名单；读不了时留着原来的。
    pub(super) fn reload_contacts(&mut self) {
        match self.store.try_contacts() {
            Ok(contacts) => self.contacts = contacts,
            Err(error) => tracing::warn!(%error, "名单读不了，先用原来的"),
        }
    }

    /// 给名字还没有首字母的对象补上（通讯录按字母分组要用，见 `memory::initial`）。
    /// 词库只有在会话那一侧才有，所以由 [`Session::open`] 把 `initial` 交进来。
    /// 一个都没缺时不写盘——键盘每次出现都会走这里，不该每次都碰 contacts.json。
    pub(in crate::session) fn fill_contact_initials(
        &mut self,
        initial: impl Fn(&str) -> Option<char>,
    ) {
        let mut filled: Vec<Contact> = Vec::new();
        for contact in &mut self.contacts {
            if contact.initial.is_some() {
                continue;
            }
            let Some(letter) = initial(&contact.name) else {
                continue;
            };
            contact.initial = Some(letter.to_string());
            filled.push(contact.clone());
        }
        for contact in filled {
            if let Err(error) = self.store.put_contact(contact) {
                tracing::warn!(code = error.code(), "首字母没写回去");
            }
        }
    }

    /// 重读当前对象的卡片与修改时间（换对象、「记一笔」、App 改了之后）；读不了时留着原来的，返回是否读成了。
    pub(super) fn reload_cards(&mut self) -> bool {
        let id = self.state.contact_id.clone();
        let loaded = match id.as_deref().map(|id| self.store.try_cards(id)) {
            None => {
                self.cards.clear();
                true
            }
            Some(Ok(cards)) => {
                self.cards = cards;
                true
            }
            Some(Err(error)) => {
                tracing::warn!(%error, "卡片读不了，先用原来的");
                false
            }
        };
        self.stamp = self.store.stamp(id.as_deref());
        if loaded {
            self.cards_stale = false;
        }
        loaded
    }

    /// 最近上屏的字与正在显示的匹配提示都清掉（键盘收起、换了输入框、换了对象）。
    pub(super) fn forget_context(&mut self) {
        self.recent.clear();
        self.current = None;
    }
}
