//! 记忆读写失败的种类。C 接口把它折成 `{"code","message"}`：code 给 Swift 分支，message 是给用户看的中文。

use thiserror::Error;

use super::{MAX_PINNED, MAX_UNPROCESSED_MATERIALS};

#[derive(Debug, Error)]
pub enum MemoryError {
    /// 全局置顶的超过 [`MAX_PINNED`] 个。
    #[error("too many pinned contacts")]
    PinLimit,

    /// 数据不合格；里面是给用户看的原因。
    #[error("invalid memory data")]
    Invalid(&'static str),

    /// 一张卡不合格：`contact` 是这个人的名字，`card` 是卡片开头几个字，`reason` 同 [`Self::Invalid`]。
    /// 单独一种是为了报错能指到卡上——用户可能是在做别的事（比如改人）时撞上一张旧的坏卡。
    #[error("invalid memory card")]
    InvalidCard {
        contact: String,

        card: String,

        reason: &'static str,
    },

    /// App 拿来写回的快照比磁盘上的旧（这期间别处改过卡片）：重读、合并后再写。
    #[error("memory changed since it was read")]
    Conflict,

    /// 这个对象没整理的素材装不下这次的几条（上限 [`MAX_UNPROCESSED_MATERIALS`]），整次不记、不悄悄丢。
    /// `remaining` 是还剩几个空位，`needed` 是这次切出的条数；键盘按它们写「这次有 2 条，这个人只剩 1 个空位」。
    #[error("too many unprocessed materials")]
    MaterialLimit { remaining: usize, needed: usize },

    /// 等 `memory/.lock` 超时（另一个进程占着）；键盘等得短，拿不到就进内存待办、下次再试。
    #[error("memory lock timed out")]
    LockTimeout,

    #[error("memory file io: {0}")]
    Io(#[from] std::io::Error),
}

impl MemoryError {
    /// `pin_limit` / `invalid` / `conflict` / `material_limit` / `lock_timeout` / `io`，与头文件里写的一致。
    pub fn code(&self) -> &'static str {
        match self {
            Self::PinLimit => "pin_limit",
            Self::Invalid(_) | Self::InvalidCard { .. } => "invalid",
            Self::Conflict => "conflict",
            Self::MaterialLimit { .. } => "material_limit",
            Self::LockTimeout => "lock_timeout",
            Self::Io(_) => "io",
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::PinLimit => format!("最多置顶 {MAX_PINNED} 个人"),
            Self::Invalid(reason) => (*reason).to_owned(),
            Self::InvalidCard {
                contact,
                card,
                reason,
            } => format!("{contact}的卡「{card}」：{reason}"),
            Self::Conflict => "记忆刚有更新，请再点一次".to_owned(),
            Self::MaterialLimit { remaining: 0, .. } => {
                format!("这个人还有 {MAX_UNPROCESSED_MATERIALS} 条没整理，先去 App 里看看")
            }
            Self::MaterialLimit { remaining, needed } => {
                format!("这次有 {needed} 条，这个人只剩 {remaining} 个空位，先去 App 里整理")
            }
            Self::LockTimeout => "记忆正被另一处使用，稍后再试".to_owned(),
            Self::Io(_) => "记忆文件读写不了（开机后还没解锁过时读不到），请解锁后重试".to_owned(),
        }
    }

    /// `{"code","message"}`；`material_limit` 另带 `remaining`（还剩几个空位）与 `needed`（这次要几条）。
    pub fn to_json(&self) -> String {
        let mut json = serde_json::json!({"code": self.code(), "message": self.message()});
        if let Self::MaterialLimit { remaining, needed } = self {
            json["remaining"] = (*remaining).into();
            json["needed"] = (*needed).into();
        }
        json.to_string()
    }
}
