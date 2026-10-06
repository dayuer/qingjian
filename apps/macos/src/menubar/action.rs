use objc2_foundation::NSInteger;

/// 分叉补丁：「素笺云」子菜单条目的 tag 区间中点；素笺云自己的 tag 是 -100..100（负数是固定动作）。
const CLOUD_AGENT_TAG_MID: NSInteger = 1100;

/// 菜单能触发的动作。编码进 NSMenuItem 的 tag，派发时再解出来。
/// 云联想的开关与模糊音都收进了「素笺云 ›」和偏好设置，顶层动作只剩窗口与版本行。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    /// 打开偏好设置窗口。
    OpenPreferences,

    /// 在访达里打开日志目录。
    OpenLogs,

    /// 打开下载页（菜单里「有新版本」那一行）。
    OpenDownload,

    /// 「素笺云」子菜单里的一项，值是素笺云菜单里的 tag，原样转给它。
    CloudAgent(isize),
}

impl MenuAction {
    pub fn tag(self) -> NSInteger {
        match self {
            Self::OpenPreferences => 2,
            Self::OpenLogs => 3,
            Self::OpenDownload => 4,
            Self::CloudAgent(tag) => CLOUD_AGENT_TAG_MID + tag,
        }
    }

    pub fn from_tag(tag: NSInteger) -> Option<Self> {
        Some(match tag {
            2 => Self::OpenPreferences,
            3 => Self::OpenLogs,
            4 => Self::OpenDownload,
            tag if (CLOUD_AGENT_TAG_MID - 100..CLOUD_AGENT_TAG_MID + 100).contains(&tag) => {
                Self::CloudAgent(tag - CLOUD_AGENT_TAG_MID)
            }
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_round_trip() {
        let all = [
            MenuAction::OpenPreferences,
            MenuAction::OpenLogs,
            MenuAction::OpenDownload,
            MenuAction::CloudAgent(-7),
            MenuAction::CloudAgent(0),
            MenuAction::CloudAgent(14),
        ];
        for action in all {
            assert_eq!(MenuAction::from_tag(action.tag()), Some(action));
        }
        assert_eq!(MenuAction::from_tag(0), None);
        assert_eq!(MenuAction::from_tag(1), None);
        assert_eq!(MenuAction::from_tag(-1), None);
    }
}
