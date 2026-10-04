//! 切场景时怎么定对象：回到这个场景上次选的人、明确不指定，或指定一个人。

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContactPick {
    /// 用这个场景上次选的人（`ScopeState::last`），没有就不指定。
    Last,

    /// 明确不指定（也会记成这个场景的「上次」）。
    Nobody,

    Contact(String),
}

impl ContactPick {
    /// C 接口的约定：空指针是 [`Self::Last`]，空字符串是 [`Self::Nobody`]，其余是对象 id。
    pub fn from_arg(arg: Option<&str>) -> Self {
        match arg {
            None => Self::Last,
            Some("") => Self::Nobody,
            Some(id) => Self::Contact(id.to_owned()),
        }
    }
}
