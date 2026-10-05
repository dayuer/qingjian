//! `contacts.json` 的一项：一个对象。名字只在这里，目录名用随机 id。两个提示开关按人设置，旧文件里没有时按开。
//! `scene` 是所属场景的 id（见 `memory/scene.rs`），`pinned_at` 是他在这个分组里被置顶的时间。
//! 代号 `display_name` 是键盘上显示的称呼（牌子、格子、提示文字都用 [`Contact::chip_name`]），App 里照旧显示名字。

use serde::{Deserialize, Serialize};

use super::Pronoun;

/// 代号最多几个字（与键盘里起名字的上限一致）。
pub const MAX_DISPLAY_NAME_CHARS: usize = 12;

/// 一个场景里最多几个置顶（键盘的选择面板先摆他们）。
pub const MAX_PINNED: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contact {
    pub id: String,

    /// 名字或代号。
    pub name: String,

    /// 键盘上显示的代号；没有时键盘显示名字。旧文件没有这个字段，键盘新建的对象也不写。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,

    /// 名字首字的拼音首字母（大写）：通讯录按字母分组与右侧索引用它（设计稿 02 的 2b）。
    /// 由键盘一侧算好写进来（词库只有它那儿有），App 只读；旧文件没有这个字段时按「#」归。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub initial: Option<String>,

    #[serde(default)]
    pub pronoun: Pronoun,

    /// 所属场景的 id（`scenes.json` 里的一项）；场景只是分组，人换场景不受限。
    pub scene: String,

    /// 置顶的时间（Unix 秒）：键盘的选择面板先摆置顶的人，同一场景最多 [`MAX_PINNED`] 个；`None` 就是没置顶。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned_at: Option<i64>,

    /// 建这个对象时的 Unix 秒（「认识 n 天」从这里算）。
    pub created_at: i64,

    /// 打字时按这个人的卡片给提示。
    #[serde(default = "true_default")]
    pub hint_on: bool,

    /// 这个人的日子与约定快到时提醒。
    #[serde(default = "true_default")]
    pub remind_on: bool,
}

impl Contact {
    /// 键盘上显示的称呼：代号去掉首尾空白后不是空的就用代号，否则用名字。
    pub fn chip_name(&self) -> &str {
        self.display_name
            .as_deref()
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .unwrap_or(&self.name)
    }

    /// 写盘前收拾代号：去掉首尾空白，全是空白的当没有。
    pub(crate) fn normalized(mut self) -> Self {
        self.display_name = self
            .display_name
            .map(|name| name.trim().to_owned())
            .filter(|name| !name.is_empty());
        self
    }
}

fn true_default() -> bool {
    true
}
