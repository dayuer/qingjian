//! 切人时怎么定当前对象：保持现在选的人、明确不指定，或指定一个人。

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContactPick {
    /// 保持现在选的人不变（幂等）。原来是「回到这个场景上次选的人」，场景去掉后没有「上次」了。
    Keep,

    /// 明确不指定。
    Nobody,

    Contact(String),
}

impl ContactPick {
    /// C 接口的约定：空指针是 [`Self::Keep`]，空字符串是 [`Self::Nobody`]，其余是对象 id。
    pub fn from_arg(arg: Option<&str>) -> Self {
        match arg {
            None => Self::Keep,
            Some("") => Self::Nobody,
            Some(id) => Self::Contact(id.to_owned()),
        }
    }
}
