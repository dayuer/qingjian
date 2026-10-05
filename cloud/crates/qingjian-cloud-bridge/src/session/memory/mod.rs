//! 会话里的本地记忆接口（C 接口在 `crate::memory::ffi`）：切当前对象、取提示、对象卡、「记一笔」（存成待整理素材）、「知道了」，
//! 以及上屏路径喂进来的最近 24 字、换输入框时清空、按修改时间重载。读-改-写都交给 `MemoryStore`（文件锁里读磁盘再写）。
//! 键盘只等 200 毫秒的锁：拿不到时「记一笔」与切人进内存待办，下次 refresh / poll / flush 或下一次记一笔时重试，主线程不卡。

mod live;
mod pending;

#[cfg(test)]
mod tests;

use qingjian_core::sentence::{LanguageModel, segment_text};

use super::Session;
use crate::entry::Entry;
use crate::memory::{
    Card, Contact, Hint, LocalDate, MaterialSource, MemoryError, Pronoun, new_id, now_unix,
    panel_cards, sanitized_scope, scope_with, split_note,
};
use crate::scope::{ContactPick, ScopeState, is_contact_id};

use self::pending::PendingNote;

pub use self::pending::DroppedNotes;

pub(super) use self::live::LiveMemory;

impl Session {
    /// 切当前对象：交给 `MemoryStore::update_contact` 在锁里重读 `state.json` 与名单，按 `pick` 定对象。
    /// 磁盘名单上没有的对象当不指定。读写失败（开机后还没解锁过）就不切，记日志；
    /// 只是拿不到锁（`LockTimeout`）时内存里照切，写盘进待办（只留最新一次，存的是按内存算好的对象）稍后重试。
    pub fn set_contact(&mut self, pick: &ContactPick) {
        let Some(memory) = self.memory.as_mut() else {
            return;
        };
        let mut deferred = false;
        let next = match memory.store.update_contact(pick, now_unix()) {
            Ok(state) => {
                memory.pending.take_scope();
                state
            }
            Err(MemoryError::LockTimeout) => {
                deferred = true;
                let wanted = scope_with(memory.state.clone(), pick, &memory.contacts);
                memory.pending.set_scope(wanted.contact_id.clone());
                wanted
            }
            Err(error) => {
                tracing::warn!(%error, "切对象没写进 state.json，不切");
                return;
            }
        };
        let moved = next.contact_id != memory.state.contact_id;
        memory.state = next;
        if moved {
            self.switch_layers(deferred);
        }
    }

    /// 当前对象；没有学习数据目录的会话没有记忆，返回 `None`。
    pub fn scope(&self) -> Option<ScopeState> {
        self.memory.as_ref().map(|memory| memory.state.clone())
    }

    /// 提示行要显示的：日子提醒优先，其次是匹配提示，各按当前对象的两个开关。
    /// 私密输入、没选对象时没有。
    pub fn memory_hint(&self) -> Option<&Hint> {
        if self.engine.is_private() {
            return None;
        }
        let memory = self.memory.as_ref()?;
        if !memory.shows_hints() {
            return None;
        }
        let contact = memory.contact()?;
        memory
            .today
            .as_ref()
            .filter(|_| contact.remind_on)
            .or_else(|| memory.current.as_ref().filter(|_| contact.hint_on))
    }

    /// 「知道了」：`today` 为真当天不再出这张卡（写进 `dismissed.json`，键盘重启也记得），为假 10 分钟内不再出。
    pub fn dismiss_hint(&mut self, card_id: &str, today: bool) {
        let Some(memory) = self.memory.as_mut() else {
            return;
        };
        memory.hints.dismiss(card_id, today, now_unix());
        if today && let Err(error) = memory.store.put_dismissed(memory.hints.dismissed()) {
            tracing::warn!(%error, "「知道了」没记下来");
        }
        if memory
            .today
            .as_ref()
            .is_some_and(|hint| hint.card_id == card_id)
        {
            memory.today = None;
        }
        if memory
            .current
            .as_ref()
            .is_some_and(|hint| hint.card_id == card_id)
        {
            memory.current = None;
        }
    }

    /// 键盘内对象卡面板：今日相关的最多 3 张（规则见 [`panel_cards`]）。
    pub fn memory_cards(&self, contact_id: &str) -> Vec<Card> {
        let Some(memory) = self.memory.as_ref() else {
            return Vec::new();
        };
        let cards = if memory.state.contact_id.as_deref() == Some(contact_id) {
            memory.cards.clone()
        } else {
            memory.store.cards(contact_id)
        };
        let focus = memory.current.as_ref().map(|hint| hint.card_id.as_str());
        panel_cards(cards, LocalDate::today(), focus)
    }

    /// 键盘「记一笔」：交给 `MemoryStore::add_material` 存成待整理素材（锁里按磁盘名单判断这个人还在、读素材失败就不写，
    /// 超过 2000 字节的切成几条，没整理的满 200 条时报 `MaterialLimit`）。不碰卡片，所以不用重读卡片与提示。
    /// 拿不到锁（`LockTimeout`）时不报错，进内存待办稍后补写（视同成功）；前面还有没写进去的就排在后面，保持顺序。
    pub fn memory_note(
        &mut self,
        contact_id: &str,
        text: &str,
        source: MaterialSource,
    ) -> Result<(), MemoryError> {
        if self.memory.is_none() {
            return Err(MemoryError::Invalid("这个键盘没有记忆目录"));
        }
        let text = text.trim();
        if text.is_empty() {
            return Err(MemoryError::Invalid("没有要记的文字"));
        }
        if !is_contact_id(contact_id) {
            return Err(MemoryError::Invalid("对象编号不对"));
        }
        self.retry_pending();
        let Some(memory) = self.memory.as_mut() else {
            return Ok(());
        };
        let now = now_unix();
        let later = PendingNote {
            contact_id: contact_id.to_owned(),
            text: text.to_owned(),
            at: now,
            source,
        };
        if memory.pending.note_count() > 0 {
            memory.pending.push_note(later);
            return Ok(());
        }
        match memory.store.add_material(contact_id, text, source, now) {
            Ok(_) => Ok(()),
            Err(MemoryError::LockTimeout) => {
                memory.pending.push_note(later);
                Ok(())
            }
            Err(error) => Err(error),
        }
    }

    /// 键盘里新建一个对象（名字与称呼），建好返回 id。和 App 一样经 `MemoryStore::put_contact` 在锁里写名单、建目录；
    /// 键盘只等 200 毫秒的锁，拿不到就报 `LockTimeout` 让用户再点一次（新建是一次性的确认，不进待办）。
    pub fn memory_add_contact(
        &mut self,
        name: &str,
        pronoun: Pronoun,
    ) -> Result<String, MemoryError> {
        let Some(memory) = self.memory.as_mut() else {
            return Err(MemoryError::Invalid("这个键盘没有记忆目录"));
        };
        let name = name.trim();
        if name.is_empty() {
            return Err(MemoryError::Invalid("名字不能是空的"));
        }
        let id = new_id()?;
        memory.store.put_contact(Contact {
            id: id.clone(),
            name: name.to_owned(),
            display_name: None,
            // 首字母由下面那次补写算（`Session::open` 里词库还在手上）；这里先留空
            initial: None,
            // 新建的人没指定技能，用设置里的默认
            skill: None,
            pronoun,
            // 新建的人不置顶，要置顶在 App 的对象设置里点
            pinned_at: None,
            created_at: now_unix(),
            hint_on: true,
            remind_on: true,
        })?;
        memory.reload_contacts();
        Ok(id)
    }

    /// 待办补写时被拒绝、没记上的条数（按原因），取完清零；键盘出现时调，提示用户一次。
    pub fn take_dropped_notes(&mut self) -> DroppedNotes {
        self.memory
            .as_mut()
            .map_or_else(DroppedNotes::default, |memory| {
                memory.pending.take_dropped()
            })
    }

    /// 重试拿不到锁时放进待办的写入：「记一笔」按顺序补写成素材（成功才出队，被拒绝的出队并记下条数与原因），再补写最新一次切人，
    /// 最后补读换对象时没读成的卡片。仍拿不到锁或读写不了就留着，这一轮到此为止（最多再等一个 200 毫秒）；
    /// 补写的切人换了叠加层返回 true。
    pub(super) fn retry_pending(&mut self) -> bool {
        let Some(memory) = self.memory.as_mut() else {
            return false;
        };
        if memory.pending.is_empty() && !memory.cards_stale {
            return false;
        }
        let mut blocked = false;
        while let Some(note) = memory.pending.front_note() {
            match memory
                .store
                .add_material(&note.contact_id, &note.text, note.source, note.at)
            {
                Ok(_) => {
                    memory.pending.pop_note();
                }
                Err(error @ (MemoryError::LockTimeout | MemoryError::Io(_))) => {
                    tracing::warn!(code = error.code(), "待写的记一笔先留着");
                    blocked = true;
                    break;
                }
                Err(error) => {
                    // 对象被忘掉、素材满了：这时没法当场告诉用户，记下条数与原因，键盘下次出现时提示（不记原话）
                    let count = split_note(&note.text).len().max(1);
                    tracing::warn!(code = error.code(), count, "待写的记一笔被拒绝，记下条数");
                    memory.pending.record_dropped(&error, count);
                    memory.pending.pop_note();
                }
            }
        }
        let mut moved = false;
        if !blocked && let Some(scope) = memory.pending.take_scope() {
            let pick = scope
                .clone()
                .map_or(ContactPick::Nobody, ContactPick::Contact);
            match memory.store.update_contact(&pick, now_unix()) {
                Ok(state) => {
                    moved = state.contact_id != memory.state.contact_id;
                    memory.state = state;
                }
                Err(error @ (MemoryError::LockTimeout | MemoryError::Io(_))) => {
                    tracing::warn!(%error, "待写的切人先留着");
                    memory.pending.restore_scope(scope);
                    blocked = true;
                }
                Err(error) => tracing::warn!(%error, "待写的切人被拒绝，丢掉"),
            }
        }
        if moved {
            self.switch_layers(false);
            return true;
        }
        let Some(memory) = self.memory.as_mut() else {
            return false;
        };
        if memory.cards_stale && !blocked {
            memory.reload_cards();
            self.rebuild_hints();
        }
        false
    }

    /// 宿主换了输入框（或键盘收起）：最近上屏的字清掉，免得在 A 聊天里打的字在 B 里触发提示。
    pub fn reset_context(&mut self) {
        if let Some(memory) = self.memory.as_mut() {
            memory.forget_context();
        }
    }

    /// 上屏路径（选词、回车原样、标点、直通的空格回车）调：私密输入时不记。
    pub(super) fn note_committed(&mut self, text: &str) {
        if self.engine.is_private() {
            return;
        }
        if let Some(memory) = self.memory.as_mut() {
            memory.recent.push_str(text);
        }
    }

    /// 每次 refresh 后：最近 24 字加当前首选去碰当前对象的卡片。
    pub(super) fn update_hint(&mut self) {
        let first = self
            .entries
            .first()
            .map(Entry::text)
            .unwrap_or_default()
            .to_owned();
        let private = self.engine.is_private();
        let Some(memory) = self.memory.as_mut() else {
            return;
        };
        let hint_on = memory.contact().is_some_and(|contact| contact.hint_on);
        if private || !memory.shows_hints() || !hint_on {
            memory.current = None;
            return;
        }
        let probe = format!("{}{first}", memory.recent.text());
        memory.current = memory.hints.match_text(&probe, now_unix());
    }

    /// 按当前对象的卡片重建索引（切词用引擎的语言模型；节流与「知道了」带过去），并算一次日子提醒。
    pub(super) fn rebuild_hints(&mut self) {
        let Some(memory) = self.memory.as_mut() else {
            return;
        };
        let model = self.engine.language_model();
        memory.hints = memory
            .hints
            .rebuild(&memory.cards, |text| words_of(text, model));
        memory.current = None;
        memory.today = memory.contact().and_then(|contact| {
            memory
                .hints
                .today(LocalDate::today(), contact.pronoun, contact.chip_name())
        });
    }

    /// `Session::poll` 开头调：`memory/` 下的文件被 App 改了（修改时间变了）就重读，读不了的留着原来的；
    /// 当前对象被删时退回不指定。换了叠加层（候选重排过）返回 true。
    pub(super) fn poll_memory(&mut self) -> bool {
        let rescoped = self.retry_pending();
        let Some(memory) = self.memory.as_mut() else {
            return false;
        };
        let stamp = memory.store.stamp(memory.state.contact_id.as_deref());
        if stamp == memory.stamp {
            return rescoped;
        }
        // 还有没写进磁盘的（锁一直被占着）：以内存里的为准，别读回旧的把刚切的切回去，也不再多等几个 200 毫秒
        if !memory.pending.is_empty() || memory.cards_stale {
            return rescoped;
        }
        memory.reload_contacts();
        let disk = match memory.store.try_state() {
            Ok(state) => state,
            Err(error) => {
                tracing::warn!(%error, "state.json 读不了，先用原来的");
                memory.state.clone()
            }
        };
        let next = sanitized_scope(disk, &memory.contacts);
        let moved = next.contact_id != memory.state.contact_id;
        memory.state = next;
        if moved {
            self.switch_layers(false);
        } else {
            memory.reload_cards();
            self.rebuild_hints();
        }
        moved || rescoped
    }

    /// 状态里选中的人变了：换叠加层、清最近的字，作废格子缓存后重读名单与卡片（`deferred` 为真说明刚拿不到锁，
    /// 不再读盘，卡片先当没有、记下待补读）、重建提示、重排候选。
    fn switch_layers(&mut self, deferred: bool) {
        let Some(memory) = self.memory.as_mut() else {
            return;
        };
        memory.handle.switch(memory.state.contact_id.as_deref());
        memory.forget_context();
        memory.cards.clear();
        memory.cards_stale = deferred;
        if !deferred {
            memory.reload_contacts();
            memory.reload_cards();
        }
        // 叠加层换了：格子缓存里的排序作废（learner_mut 会清缓存），这个人的用户词快照重建
        self.engine.learner_mut().scope_changed();
        self.rebuild_hints();
        self.refresh_candidates();
    }
}

/// 卡片文字切成词；没有语言模型（或一个词都不认识）时切不出来，只靠关键词。
fn words_of(text: &str, model: &dyn LanguageModel) -> Vec<String> {
    segment_text(text, model)
        .into_iter()
        .flatten()
        .flatten()
        .collect()
}
