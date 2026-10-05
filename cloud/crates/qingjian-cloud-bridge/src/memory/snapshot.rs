//! App 整份读写的 JSON：`{"contacts":[…],"cards":{id:[…]},"revs":{id:n},"state":{…},"broken":[id…],"unassigned":[…]}`。
//! `revs` 是读的时候各对象 `cards.json` 的修订号，写回时拿来判断这期间键盘有没有改过。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::{Card, Contact};
use crate::scope::ScopeState;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MemorySnapshot {
    pub contacts: Vec<Contact>,

    /// 对象 id → 卡片。
    pub cards: BTreeMap<String, Vec<Card>>,

    /// 对象 id → 读时 `cards.json` 的修订号；没有的按 0。
    pub revs: BTreeMap<String, u64>,

    /// 键盘当前的场景与对象，只给 App 显示；写回时忽略。
    pub state: ScopeState,

    /// 这次读时卡片文件坏了、已改名备份的对象（App 据此提示）；写回时忽略。
    pub broken: Vec<String>,

    /// 「还没归到人的」卡片：App 首页「+ 记一条」先快速记下（不问是谁），事后「补上」归到某个人。
    /// 只有 App 写，键盘不碰，所以不带修订号、不走冲突那套。
    pub unassigned: Vec<Card>,
}
