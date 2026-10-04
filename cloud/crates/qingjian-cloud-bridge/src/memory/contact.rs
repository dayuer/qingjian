//! `contacts.json` 的一项：一个对象。名字只在这里，目录名用随机 id。两个提示开关按人设置，旧文件里没有时按开。

use qingjian_cloud_proto::Scene;
use serde::{Deserialize, Serialize};

use super::Pronoun;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contact {
    pub id: String,

    /// 名字或代号。
    pub name: String,

    #[serde(default)]
    pub pronoun: Pronoun,

    pub scene: Scene,

    /// 建这个对象时的 Unix 秒（「认识 n 天」从这里算）。
    pub created_at: i64,

    /// 打字时按这个人的卡片给提示。
    #[serde(default = "true_default")]
    pub hint_on: bool,

    /// 这个人的日子与约定快到时提醒。
    #[serde(default = "true_default")]
    pub remind_on: bool,
}

fn true_default() -> bool {
    true
}
