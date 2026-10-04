//! 记忆读写失败的种类。C 接口把它折成 `{"code","message"}`：code 给 Swift 分支，message 是给用户看的中文。

use thiserror::Error;

use qingjian_cloud_proto::Scene;

use super::MAX_CONTACTS;
use crate::scope::scene_label;

#[derive(Debug, Error)]
pub enum MemoryError {
    /// 这个场景的对象超过上限（每个场景各自计数）。
    #[error("too many contacts in the {0:?} scene")]
    ContactLimit(Scene),

    /// 数据不合格；里面是给用户看的原因。
    #[error("invalid memory data")]
    Invalid(&'static str),

    /// App 拿来写回的快照比磁盘上的旧（这期间键盘「记一笔」改过）：重读、合并后再写。
    #[error("memory changed since it was read")]
    Conflict,

    /// 等 `memory/.lock` 超时（另一个进程占着）；键盘等得短，拿不到就进内存待办、下次再试。
    #[error("memory lock timed out")]
    LockTimeout,

    #[error("memory file io: {0}")]
    Io(#[from] std::io::Error),
}

impl MemoryError {
    /// `contact_limit` / `invalid` / `conflict` / `lock_timeout` / `io`，与头文件里写的一致。
    pub fn code(&self) -> &'static str {
        match self {
            Self::ContactLimit(_) => "contact_limit",
            Self::Invalid(_) => "invalid",
            Self::Conflict => "conflict",
            Self::LockTimeout => "lock_timeout",
            Self::Io(_) => "io",
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::ContactLimit(scene) => format!("{}最多 {MAX_CONTACTS} 个人", scene_label(*scene)),
            Self::Invalid(reason) => (*reason).to_owned(),
            Self::Conflict => "记忆刚在键盘里改过，已重新读取".to_owned(),
            Self::LockTimeout => "记忆正被另一处使用，稍后再试".to_owned(),
            Self::Io(_) => "记忆文件读写不了（开机后还没解锁过时读不到），请解锁后重试".to_owned(),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::json!({"code": self.code(), "message": self.message()}).to_string()
    }
}
