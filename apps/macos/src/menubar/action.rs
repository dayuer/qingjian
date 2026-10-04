use objc2_foundation::NSInteger;
use qingjian_core::FuzzyRules;

/// 模糊音条目的 tag 起点，后面加规则在 [`FuzzyRules::NAMES`] 里的下标。
const FUZZY_TAG_BASE: NSInteger = 100;

/// 分叉补丁：「素笺云」子菜单条目的 tag 区间中点；素笺云自己的 tag 是 -100..100（负数是固定动作）。
const CLOUD_AGENT_TAG_MID: NSInteger = 1100;

/// 菜单能触发的动作。编码进 NSMenuItem 的 tag，派发时再解出来。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    /// 开关云联想（写 `[predict] enabled`）。
    ToggleCloud,

    /// 开关一条模糊音规则，值是 [`FuzzyRules::NAMES`] 的下标。
    ToggleFuzzy(usize),

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
            Self::ToggleCloud => 1,
            Self::OpenPreferences => 2,
            Self::OpenLogs => 3,
            Self::OpenDownload => 4,
            Self::ToggleFuzzy(index) => FUZZY_TAG_BASE + index as NSInteger,
            Self::CloudAgent(tag) => CLOUD_AGENT_TAG_MID + tag,
        }
    }

    pub fn from_tag(tag: NSInteger) -> Option<Self> {
        Some(match tag {
            1 => Self::ToggleCloud,
            2 => Self::OpenPreferences,
            3 => Self::OpenLogs,
            4 => Self::OpenDownload,
            tag if (CLOUD_AGENT_TAG_MID - 100..CLOUD_AGENT_TAG_MID + 100).contains(&tag) => {
                Self::CloudAgent(tag - CLOUD_AGENT_TAG_MID)
            }
            _ => {
                let index = usize::try_from(tag.checked_sub(FUZZY_TAG_BASE)?).ok()?;
                (index < FuzzyRules::NAMES.len()).then_some(Self::ToggleFuzzy(index))?
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_round_trip() {
        let all = [
            MenuAction::ToggleCloud,
            MenuAction::OpenPreferences,
            MenuAction::OpenLogs,
            MenuAction::OpenDownload,
            MenuAction::ToggleFuzzy(0),
            MenuAction::ToggleFuzzy(FuzzyRules::NAMES.len() - 1),
            MenuAction::CloudAgent(-7),
            MenuAction::CloudAgent(0),
            MenuAction::CloudAgent(14),
        ];
        for action in all {
            assert_eq!(MenuAction::from_tag(action.tag()), Some(action));
        }
        assert_eq!(MenuAction::from_tag(0), None);
        assert_eq!(
            MenuAction::from_tag(FUZZY_TAG_BASE + FuzzyRules::NAMES.len() as NSInteger),
            None
        );
        assert_eq!(MenuAction::from_tag(-1), None);
    }
}
