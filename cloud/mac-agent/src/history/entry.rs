//! 历史里的一条剪贴板。

#[derive(Debug, Clone)]
pub struct Entry {
    /// 服务端序号，删除时用。
    pub seq: u64,

    /// 来自哪台设备。
    pub device: String,

    /// 是本机复制的。
    pub mine: bool,

    pub text: String,
}

/// 菜单里每条最多显示多少个字符。
const TITLE_CHARS: usize = 40;

impl Entry {
    /// 菜单标题：`[设备] 前 40 个字…`，换行压成空格。
    pub fn title(&self) -> String {
        let flat: String = self
            .text
            .chars()
            .map(|c| if c.is_whitespace() { ' ' } else { c })
            .collect();
        let flat = flat.trim();
        let mut title: String = flat.chars().take(TITLE_CHARS).collect();
        if flat.chars().count() > TITLE_CHARS {
            title.push('…');
        }
        let source = if self.mine {
            "本机"
        } else {
            self.device.as_str()
        };
        format!("[{source}] {title}")
    }
}
