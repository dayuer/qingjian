//! `memory/state.json`：键盘当前选的人、各人上次被选的时间（键盘写，App 不改）。
//! 提示开关在各个对象上（`Contact`），不在这里。2026-10-05 起没有「场景」，这个文件里也就没有场景相关的东西。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScopeState {
    /// 当前对象。
    pub contact_id: Option<String>,

    /// 对象 id → 上次在键盘里选中这个人的 Unix 秒（列人时按沟通情况排用）；旧文件没有时为空。
    pub used: BTreeMap<String, i64>,
}
