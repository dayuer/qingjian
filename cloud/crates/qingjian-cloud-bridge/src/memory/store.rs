//! `memory/` 下的文件：`contacts.json`、`state.json`、`dismissed.json`、`<对象 id>/cards.json`（素材 `materials.jsonl` 在 `materials/file.rs`）。
//! App 与键盘是两个进程，都会读-改-写，所以每个操作都在 `memory/.lock` 的文件锁（flock）里完成，读也在锁里；
//! 写走 `cloud_config::write_atomic`（同目录临时文件加改名），而且不建父目录：对象目录只在建对象时创建。
//! 解析不了的文件改名为 `<文件>.broken-<unix 秒>` 再按空处理；读不了的（开机后还没解锁过时的数据保护、权限）不改名，读-改-写直接报错，
//! 不拿空表覆盖真文件。只读的 `contacts()` / `cards()` / `state()` 读不了时给空，只给显示用，不能接着写。

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs::{File, OpenOptions, TryLockError};
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use qingjian_cloud_proto::{MAX_CARD_KEYWORDS, MAX_CARD_TEXT_CHARS, Scene};
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::{
    Card, CardsFile, Contact, DismissedFile, LocalDate, MAX_CONTACTS, MAX_DISPLAY_NAME_CHARS,
    MEMORY_DIR, MemoryError, MemorySnapshot, now_unix, sanitized_scope, scope_with,
};
use crate::cloud_config::write_atomic;
use crate::scope::{ContactPick, ScopeState, is_contact_id};

const CONTACTS_FILE: &str = "contacts.json";

const STATE_FILE: &str = "state.json";

const CARDS_FILE: &str = "cards.json";

const DISMISSED_FILE: &str = "dismissed.json";

const LOCK_FILE: &str = ".lock";

/// 等文件锁缺省最多多久（App 用）；另一个进程正常只占几毫秒。
pub const DEFAULT_LOCK_TIMEOUT: Duration = Duration::from_secs(2);

/// 键盘等锁的上限：键盘主线程上不能卡，拿不到就进内存待办、下次刷新再试。
pub const KEYBOARD_LOCK_TIMEOUT: Duration = Duration::from_millis(200);

const LOCK_RETRY: Duration = Duration::from_millis(5);

/// 一个关键词至少、至多几个字（spec 对云端卡关键词的校验，手写卡一并按它）。
const MIN_KEYWORD_CHARS: usize = 2;

const MAX_KEYWORD_CHARS: usize = 8;

/// 「知道了」的记录留多少天。
const DISMISSED_KEEP_DAYS: i64 = 30;

#[derive(Debug, Clone)]
pub struct MemoryStore {
    /// `<学习数据目录>/memory`。
    root: PathBuf,

    /// 等 `.lock` 的上限，超时返回 `MemoryError::LockTimeout`。
    lock_timeout: Duration,
}

impl MemoryStore {
    /// 等锁最多 [`DEFAULT_LOCK_TIMEOUT`]（App 用）。
    pub fn open(user_dir: &Path) -> Self {
        Self::open_with_lock_timeout(user_dir, DEFAULT_LOCK_TIMEOUT)
    }

    /// 自定等锁上限；键盘用 [`KEYBOARD_LOCK_TIMEOUT`]。
    pub fn open_with_lock_timeout(user_dir: &Path, lock_timeout: Duration) -> Self {
        Self {
            root: user_dir.join(MEMORY_DIR),
            lock_timeout,
        }
    }

    pub fn lock_timeout(&self) -> Duration {
        self.lock_timeout
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// 名单；读不了时为空（只给显示用）。
    pub fn contacts(&self) -> Vec<Contact> {
        self.try_contacts().unwrap_or_default()
    }

    pub fn try_contacts(&self) -> Result<Vec<Contact>, MemoryError> {
        let _lock = self.lock()?;
        self.read_contacts()
    }

    /// 一个人的卡片；读不了时为空（只给显示用）。
    pub fn cards(&self, contact_id: &str) -> Vec<Card> {
        self.try_cards(contact_id).unwrap_or_default()
    }

    pub fn try_cards(&self, contact_id: &str) -> Result<Vec<Card>, MemoryError> {
        if !is_contact_id(contact_id) {
            return Err(MemoryError::Invalid("对象编号不对"));
        }
        let _lock = self.lock()?;
        Ok(self.read_cards(contact_id)?.0.cards)
    }

    /// 当前场景与对象；读不了时按缺省（只给显示用）。
    pub fn state(&self) -> ScopeState {
        self.try_state().unwrap_or_default()
    }

    pub fn try_state(&self) -> Result<ScopeState, MemoryError> {
        let _lock = self.lock()?;
        Ok(read_json(&self.state_path())?.0)
    }

    /// 加一个对象或改已有的（同 id，场景不能改）。这个场景超过 [`MAX_CONTACTS`] 个返回 [`MemoryError::ContactLimit`]。
    /// 对象目录在这里建，别处写卡片都不建目录。
    pub fn put_contact(&self, contact: Contact) -> Result<(), MemoryError> {
        let contact = contact.normalized();
        validate_contacts(std::slice::from_ref(&contact))?;
        let _lock = self.lock()?;
        let mut contacts = self.read_contacts()?;
        check_scenes_kept(&contacts, std::slice::from_ref(&contact))?;
        let id = contact.id.clone();
        match contacts.iter_mut().find(|c| c.id == contact.id) {
            Some(existing) => *existing = contact,
            None => contacts.push(contact),
        }
        check_limit(&contacts)?;
        std::fs::create_dir_all(self.root.join(&id))?;
        write_json(&self.contacts_path(), &contacts)
    }

    /// 忘掉一个人：先删 `memory/<id>/` 整个目录（卡片与分区学习），删成了再从名单去掉。
    pub fn forget_contact(&self, id: &str) -> Result<(), MemoryError> {
        if !is_contact_id(id) {
            return Err(MemoryError::Invalid("对象编号不对"));
        }
        let _lock = self.lock()?;
        let mut contacts = self.read_contacts()?;
        remove_dir(&self.root.join(id))?;
        contacts.retain(|c| c.id != id);
        write_json(&self.contacts_path(), &contacts)
    }

    /// 整份换掉一个人的卡片（修订号加一）。名单上（磁盘上的）没有这个人就报错。
    pub fn put_cards(&self, contact_id: &str, cards: &[Card]) -> Result<(), MemoryError> {
        if !is_contact_id(contact_id) {
            return Err(MemoryError::Invalid("对象编号不对"));
        }
        validate_cards(cards)?;
        let _lock = self.lock()?;
        self.require_contact(contact_id)?;
        let (file, _) = self.read_cards(contact_id)?;
        self.write_cards(contact_id, file.rev + 1, cards)
    }

    /// 键盘切场景与对象：在锁里重读 `state.json` 与名单，按 `pick` 定对象（`Last` 回到这个场景上次选的人）；
    /// 对象不在磁盘名单上或不是这个场景的人就当不指定；切到了某人时把 `now` 记进 `used`。
    pub fn update_scope(
        &self,
        scene: Scene,
        pick: &ContactPick,
        now: i64,
    ) -> Result<ScopeState, MemoryError> {
        let _lock = self.lock()?;
        let contacts = self.read_contacts()?;
        let (state, _): (ScopeState, bool) = read_json(&self.state_path())?;
        let mut state = scope_with(state, scene, pick, &contacts);
        if let Some(id) = state.contact_id.clone() {
            state.used.insert(id, now);
        }
        write_json(&self.state_path(), &state)?;
        Ok(state)
    }

    /// App 读的整份数据，带各对象的修订号；卡片文件坏了的对象记进 `broken`。任何一个文件读不了就整份报错。
    pub fn snapshot(&self) -> Result<MemorySnapshot, MemoryError> {
        let _lock = self.lock()?;
        let contacts = self.read_contacts()?;
        let mut snapshot = MemorySnapshot {
            state: read_json(&self.state_path())?.0,
            ..MemorySnapshot::default()
        };
        for contact in contacts.iter().filter(|c| is_contact_id(&c.id)) {
            let (file, quarantined) = self.read_cards(&contact.id)?;
            if quarantined {
                snapshot.broken.push(contact.id.clone());
            }
            snapshot.revs.insert(contact.id.clone(), file.rev);
            snapshot.cards.insert(contact.id.clone(), file.cards);
        }
        snapshot.contacts = contacts;
        Ok(snapshot)
    }

    /// App 整份写回。先校验（已有的人不能换场景）；锁里逐个比修订号，磁盘上比快照新（键盘这期间改过）就整份不写、返回 [`MemoryError::Conflict`]；
    /// 名单上没了的人连目录一起删；只重写卡片有变化的对象（修订号加一）；`state` 不采纳，当前对象被删了就置空。
    pub fn write_snapshot(&self, snapshot: &MemorySnapshot) -> Result<(), MemoryError> {
        let contacts: Vec<Contact> = snapshot
            .contacts
            .iter()
            .cloned()
            .map(Contact::normalized)
            .collect();
        validate_contacts(&contacts)?;
        check_limit(&contacts)?;
        for (id, cards) in &snapshot.cards {
            if !contacts.iter().any(|c| &c.id == id) {
                return Err(MemoryError::Invalid("卡片对不上人"));
            }
            validate_cards(cards)?;
        }
        let _lock = self.lock()?;
        let old = self.read_contacts()?;
        check_scenes_kept(&old, &contacts)?;
        let mut changed = Vec::new();
        for (id, cards) in &snapshot.cards {
            let (disk, _) = self.read_cards(id)?;
            if disk.rev > snapshot.revs.get(id).copied().unwrap_or(0) {
                return Err(MemoryError::Conflict);
            }
            if disk.cards != *cards {
                changed.push((id, disk.rev + 1, cards));
            }
        }
        for gone in old
            .iter()
            .filter(|o| is_contact_id(&o.id) && !contacts.iter().any(|c| c.id == o.id))
        {
            remove_dir(&self.root.join(&gone.id))?;
        }
        for contact in &contacts {
            std::fs::create_dir_all(self.root.join(&contact.id))?;
        }
        for (id, rev, cards) in changed {
            self.write_cards(id, rev, cards)?;
        }
        write_json(&self.contacts_path(), &contacts)?;
        let (state, _): (ScopeState, bool) = read_json(&self.state_path())?;
        let fixed = sanitized_scope(state.clone(), &contacts);
        if fixed != state {
            write_json(&self.state_path(), &fixed)?;
        }
        Ok(())
    }

    /// 「知道了」的记录，丢掉 30 天前的与不在 `known` 里的卡；读不了时为空。
    pub fn dismissed(
        &self,
        today: LocalDate,
        known: &HashSet<String>,
    ) -> HashMap<String, LocalDate> {
        let read = || -> Result<DismissedFile, MemoryError> {
            let _lock = self.lock()?;
            Ok(read_json(&self.dismissed_path())?.0)
        };
        let file = read().unwrap_or_else(|error| {
            tracing::warn!(%error, "「知道了」的记录读不了");
            DismissedFile::default()
        });
        file.cards
            .into_iter()
            .filter(|(id, _)| known.contains(id))
            .filter_map(|(id, day)| Some((id, LocalDate::parse(&day)?)))
            .filter(|(_, day)| day.days_until(today) <= DISMISSED_KEEP_DAYS)
            .collect()
    }

    pub fn put_dismissed(&self, dismissed: &HashMap<String, LocalDate>) -> Result<(), MemoryError> {
        let file = DismissedFile {
            cards: dismissed
                .iter()
                .map(|(id, day)| (id.clone(), day.to_string()))
                .collect::<BTreeMap<_, _>>(),
        };
        let _lock = self.lock()?;
        write_json(&self.dismissed_path(), &file)
    }

    /// `contacts.json`、`state.json` 与当前对象 `cards.json` 的修改时间；键盘轮询时比对，变了才重读。
    pub fn stamp(&self, contact_id: Option<&str>) -> [Option<SystemTime>; 3] {
        let cards = contact_id
            .filter(|id| is_contact_id(id))
            .map(|id| self.cards_path(id));
        [
            modified(&self.contacts_path()),
            modified(&self.state_path()),
            cards.as_deref().and_then(modified),
        ]
    }

    /// 拿 `memory/.lock` 的文件锁：`try_lock` 加重试，最多等 `lock_timeout`。返回的文件关掉时锁就放了。
    /// flock 锁的是打开的文件，同一进程里两个 `MemoryStore` 也互斥。
    pub(super) fn lock(&self) -> Result<File, MemoryError> {
        std::fs::create_dir_all(&self.root)?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(self.root.join(LOCK_FILE))?;
        let started = Instant::now();
        loop {
            match file.try_lock() {
                Ok(()) => return Ok(file),
                Err(TryLockError::WouldBlock) if started.elapsed() < self.lock_timeout => {
                    std::thread::sleep(LOCK_RETRY);
                }
                Err(TryLockError::WouldBlock) => {
                    return Err(MemoryError::LockTimeout);
                }
                Err(TryLockError::Error(error)) => return Err(error.into()),
            }
        }
    }

    pub(super) fn require_contact(&self, contact_id: &str) -> Result<(), MemoryError> {
        if self.read_contacts()?.iter().any(|c| c.id == contact_id) {
            Ok(())
        } else {
            Err(MemoryError::Invalid("名单上没有这个人"))
        }
    }

    fn read_contacts(&self) -> Result<Vec<Contact>, MemoryError> {
        read_json(&self.contacts_path()).map(|(contacts, _)| contacts)
    }

    fn read_cards(&self, contact_id: &str) -> Result<(CardsFile, bool), MemoryError> {
        read_with(&self.cards_path(contact_id), parse_cards)
    }

    fn write_cards(&self, contact_id: &str, rev: u64, cards: &[Card]) -> Result<(), MemoryError> {
        let file = CardsFile {
            rev,
            cards: cards.to_vec(),
        };
        write_json(&self.cards_path(contact_id), &file)
    }

    fn contacts_path(&self) -> PathBuf {
        self.root.join(CONTACTS_FILE)
    }

    fn state_path(&self) -> PathBuf {
        self.root.join(STATE_FILE)
    }

    fn dismissed_path(&self) -> PathBuf {
        self.root.join(DISMISSED_FILE)
    }

    fn cards_path(&self, contact_id: &str) -> PathBuf {
        self.root.join(contact_id).join(CARDS_FILE)
    }
}

/// `cards.json`：现在是 `{"rev","cards"}`，早期是卡片数组（当修订号 0）。
fn parse_cards(text: &str) -> Result<CardsFile, serde_json::Error> {
    serde_json::from_str::<CardsFile>(text).or_else(|error| {
        serde_json::from_str::<Vec<Card>>(text)
            .map(|cards| CardsFile { rev: 0, cards })
            .map_err(|_| error)
    })
}

fn read_json<T: DeserializeOwned + Default>(path: &Path) -> Result<(T, bool), MemoryError> {
    read_with(path, |text| serde_json::from_str(text))
}

/// 读一个 JSON 文件：不在按缺省；解析不了改名备份后按缺省，第二个值为真；其余 io 错误原样返回。
fn read_with<T: Default>(
    path: &Path,
    parse: impl Fn(&str) -> Result<T, serde_json::Error>,
) -> Result<(T, bool), MemoryError> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok((T::default(), false)),
        Err(error) => return Err(error.into()),
    };
    match parse(&text) {
        Ok(value) => Ok((value, false)),
        Err(error) => {
            tracing::warn!(path = %path.display(), %error, "记忆文件坏了，改名备份后按空处理");
            quarantine(path);
            Ok((T::default(), true))
        }
    }
}

fn write_json<T: Serialize + ?Sized>(path: &Path, value: &T) -> Result<(), MemoryError> {
    let bytes =
        serde_json::to_vec_pretty(value).map_err(|_| MemoryError::Invalid("数据编码失败"))?;
    write_atomic(path, &bytes, false)?;
    Ok(())
}

pub(super) fn quarantine(path: &Path) {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".broken-{}", now_unix()));
    if let Err(error) = std::fs::rename(path, path.with_file_name(name)) {
        tracing::warn!(%error, "坏文件没改成备份名");
    }
}

fn remove_dir(dir: &Path) -> Result<(), MemoryError> {
    match std::fs::remove_dir_all(dir) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn modified(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|m| m.modified()).ok()
}

/// 每个场景各自最多 [`MAX_CONTACTS`] 个，互不相通。
fn check_limit(contacts: &[Contact]) -> Result<(), MemoryError> {
    for scene in [Scene::Dating, Scene::Daily, Scene::Work] {
        if contacts.iter().filter(|c| c.scene == scene).count() > MAX_CONTACTS {
            return Err(MemoryError::ContactLimit(scene));
        }
    }
    Ok(())
}

/// 对象建好后不能换场景：分区学习与卡片都按场景走，要换就忘掉再建。
fn check_scenes_kept(old: &[Contact], new: &[Contact]) -> Result<(), MemoryError> {
    let moved = new
        .iter()
        .any(|n| old.iter().any(|o| o.id == n.id && o.scene != n.scene));
    if moved {
        return Err(MemoryError::Invalid("换场景需要忘掉后重新加"));
    }
    Ok(())
}

fn validate_contacts(contacts: &[Contact]) -> Result<(), MemoryError> {
    let mut seen = HashSet::new();
    for contact in contacts {
        if !is_contact_id(&contact.id) {
            return Err(MemoryError::Invalid("对象编号不对"));
        }
        if contact.name.trim().is_empty() {
            return Err(MemoryError::Invalid("名字不能是空的"));
        }
        if contact
            .display_name
            .as_deref()
            .is_some_and(|name| name.trim().chars().count() > MAX_DISPLAY_NAME_CHARS)
        {
            return Err(MemoryError::Invalid("键盘上的代号最多 12 个字"));
        }
        if !seen.insert(contact.id.as_str()) {
            return Err(MemoryError::Invalid("同一个人出现了两次"));
        }
    }
    Ok(())
}

fn validate_cards(cards: &[Card]) -> Result<(), MemoryError> {
    let mut seen = HashSet::new();
    for card in cards {
        if !is_contact_id(&card.id) {
            return Err(MemoryError::Invalid("卡片编号不对"));
        }
        if card.text.trim().is_empty() {
            return Err(MemoryError::Invalid("卡片内容不能是空的"));
        }
        if card.text.chars().count() > MAX_CARD_TEXT_CHARS {
            return Err(MemoryError::Invalid("一张卡最多 200 个字"));
        }
        if card.keywords.len() > MAX_CARD_KEYWORDS {
            return Err(MemoryError::Invalid("关键词最多 8 个"));
        }
        if card.keywords.iter().any(|keyword| {
            !(MIN_KEYWORD_CHARS..=MAX_KEYWORD_CHARS).contains(&keyword.trim().chars().count())
        }) {
            return Err(MemoryError::Invalid("每个关键词要 2 到 8 个字"));
        }
        if card
            .when
            .as_deref()
            .is_some_and(|when| LocalDate::parse(when).is_none())
        {
            return Err(MemoryError::Invalid("日期要写成 2026-10-04 这样"));
        }
        if !seen.insert(card.id.as_str()) {
            return Err(MemoryError::Invalid("同一张卡片出现了两次"));
        }
    }
    Ok(())
}
