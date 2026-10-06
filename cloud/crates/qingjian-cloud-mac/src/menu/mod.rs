//! 菜单内容：连接状态、账号（登录 / 四个开关 / 退出登录）、学习数据状态、最近的剪贴板（点一条复制到本机）、暂停与几个操作。
//! 只算出行列表，由输入法画成「中☁ → 素笺云 ›」子菜单（输入法只认文字行与可点项，开关画成「名字：开 / 关」）。
//! tag 区分动作：非负数是历史条目的下标，负数是固定动作（输入法转发 -100..100 的 tag）。

mod account_menu;
mod display;
mod lines;

use qingjian_cloud_client::{DataStatus, Status};
use qingjian_cloud_proto::{Consents, Feature};

use crate::history::History;

pub use account_menu::AccountMenu;
pub use display::Display;
pub use lines::Line;

pub const TAG_PAUSE: isize = -1;
pub const TAG_RELOAD: isize = -2;
pub const TAG_OPEN_CONFIG: isize = -3;
pub const TAG_SYNC_NOW: isize = -6;
pub const TAG_CREATE_SPACE: isize = -7;
pub const TAG_SIGN_OUT: isize = -8;
pub const TAG_JOIN_WITH_CODE: isize = -9;
pub const TAG_CANCEL_JOIN: isize = -17;
pub const TAG_CLEAR_INPUT_LOG: isize = -16;

/// 菜单里显示的功能开关（标题与 iOS 的功能清单一字一样）；云端记忆与同步打字内容
/// 打开前要先过同意说明（见 service 里的确认弹窗）。
const MENU_FEATURES: [Feature; 5] = [
    Feature::Memory,
    Feature::InputLog,
    Feature::Sync,
    Feature::Clipboard,
    Feature::Llm,
];

/// 五个功能开关的 tag：-10 起按 [`MENU_FEATURES`] 的顺序往下排（-10..=-14）；其余固定动作避开这一段（清空 -16、取消加入 -17）。
const TAG_TOGGLE_FIRST: isize = -10;

pub fn toggle_tag(feature: Feature) -> isize {
    let index = MENU_FEATURES
        .iter()
        .position(|&f| f == feature)
        .unwrap_or(MENU_FEATURES.len());
    TAG_TOGGLE_FIRST - index as isize
}

pub fn toggled_feature(tag: isize) -> Option<Feature> {
    let index = usize::try_from(TAG_TOGGLE_FIRST - tag).ok()?;
    MENU_FEATURES.get(index).copied()
}

pub fn build_lines(
    display: &Display,
    account: &AccountMenu,
    data: Option<&DataStatus>,
    history: &History,
) -> Vec<Line> {
    let mut lines = vec![Line::Text(status_line(display))];
    // 学习数据的行只在同步着的时候显示：所有项都被服务器停了（或没登录）就没有「刚刚同步」可说
    let data_line = data.and_then(|data| data_line(data, account.consents));
    if let Some(line) = &data_line {
        lines.push(Line::Text(line.clone()));
    }
    if let Some(note) = &account.note {
        lines.push(Line::Text(short(note)));
    }
    lines.push(Line::Separator);
    if account.signed_in {
        for feature in MENU_FEATURES {
            let state = if account.consents.get(feature) {
                "开"
            } else {
                "关"
            };
            lines.push(Line::Action(
                format!("{}：{state}", feature_title(feature)),
                toggle_tag(feature),
            ));
        }
        lines.push(Line::Separator);
        if account.consents.clipboard {
            let before = lines.len();
            for (index, entry) in history.entries().enumerate() {
                lines.push(Line::Action(entry.title(), index as isize));
            }
            if lines.len() == before {
                lines.push(Line::Text("还没有剪贴板记录".to_owned()));
            }
            lines.push(Line::Separator);
        }
        let pause = if matches!(display, Display::Paused) {
            "继续同步"
        } else {
            "暂停同步"
        };
        lines.push(Line::Action(pause.to_owned(), TAG_PAUSE));
        if data_line.is_some() {
            lines.push(Line::Action("立即同步学习数据".to_owned(), TAG_SYNC_NOW));
        }
        lines.push(Line::Action(
            "退出登录（解绑这台 Mac）".to_owned(),
            TAG_SIGN_OUT,
        ));
        lines.push(Line::Action(
            "清空云端输入记录…".to_owned(),
            TAG_CLEAR_INPUT_LOG,
        ));
    } else if account.signing_in {
        lines.push(Line::Text("正在加入…".to_owned()));
        lines.push(Line::Action("取消".to_owned(), TAG_CANCEL_JOIN));
    } else {
        // 手机上多半已经有空间：引导走匹配码加入（手机出码，这里输码，手机上允许）
        lines.push(Line::Action(
            "输入匹配码加入…".to_owned(),
            TAG_JOIN_WITH_CODE,
        ));
        lines.push(Line::Action("开通素笺云".to_owned(), TAG_CREATE_SPACE));
    }
    lines.push(Line::Action("重新加载配置".to_owned(), TAG_RELOAD));
    lines.push(Line::Action("打开配置文件…".to_owned(), TAG_OPEN_CONFIG));
    lines
}

pub fn status_line(display: &Display) -> String {
    match display {
        Display::Unconfigured(reason) => short(reason),
        Display::SignedOut => "未登录".to_owned(),
        Display::SignedIn => "已登录".to_owned(),
        Display::Paused => "已暂停同步".to_owned(),
        Display::Sync { status, pending } => {
            let base = match status {
                Status::Connecting => "正在连接…".to_owned(),
                Status::Online => "已连接".to_owned(),
                Status::Offline(error) => format!("离线，稍后自动重试（{}）", short(error)),
                Status::Unauthorized => "登录已失效，请重新登录".to_owned(),
                Status::Disabled => "跨设备剪贴板在服务器上没开".to_owned(),
            };
            if *pending > 0 {
                format!("{base} · {pending} 条待上传")
            } else {
                base
            }
        }
    }
}

fn feature_title(feature: Feature) -> &'static str {
    match feature {
        Feature::Clipboard => "跨设备剪贴板",
        Feature::Sync => "同步学习数据与设置",
        Feature::InputLog => "同步打字内容",
        Feature::Llm => "大模型（润色、云联想）",
        Feature::Memory => "云端记忆（把记下的素材整理成卡）",
    }
}

/// 学习数据那一行；开着的几项都被服务器停了时为 `None`（对应开关马上会被记成关）。
fn data_line(data: &DataStatus, consents: Consents) -> Option<String> {
    let enabled = [
        (Feature::Sync, consents.sync),
        (Feature::InputLog, consents.input_log),
    ];
    let running = enabled
        .iter()
        .filter(|(feature, on)| *on && !data.disabled.contains(feature))
        .count();
    if running == 0 {
        return None;
    }
    let mut line = if data.unauthorized {
        "学习数据：登录已失效".to_owned()
    } else if let Some(error) = &data.error {
        format!("学习数据：同步失败，稍后重试（{}）", short(error))
    } else if data.waiting_for_ime {
        "学习数据：等输入法合并（切到素笺打几个字）".to_owned()
    } else if let Some(ms) = data.last_ok_ms {
        format!("学习数据：{}同步", ago(ms))
    } else {
        "学习数据：正在同步…".to_owned()
    };
    if data.config_conflict {
        line.push_str(" · 设置有冲突，旧的一份已备份");
    }
    Some(line)
}

/// 「刚刚」/「N 分钟前」/「N 小时前」。
fn ago(ms: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default();
    let minutes = (now - ms).max(0) / 60_000;
    match minutes {
        0 => "刚刚".to_owned(),
        1..=59 => format!("{minutes} 分钟前"),
        _ => format!("{} 小时前", minutes / 60),
    }
}

/// 错误信息在菜单里只显示前 40 个字符，完整的在日志里。
fn short(text: &str) -> String {
    text.chars().take(40).collect()
}

#[cfg(test)]
mod tests {
    use qingjian_cloud_proto::Consents;

    use super::*;

    fn account(signed_in: bool) -> AccountMenu {
        AccountMenu {
            signed_in,
            signing_in: false,
            consents: Consents {
                sync: true,
                ..Consents::default()
            },
            note: None,
        }
    }

    #[test]
    fn toggle_tags_round_trip_and_stay_in_range() {
        for feature in MENU_FEATURES {
            let tag = toggle_tag(feature);
            assert!((-100..0).contains(&tag));
            assert_eq!(toggled_feature(tag), Some(feature));
        }
        // 五项都在菜单里
        assert_eq!(
            toggled_feature(toggle_tag(Feature::Memory)),
            Some(Feature::Memory)
        );
        assert_eq!(
            toggled_feature(toggle_tag(Feature::Llm)),
            Some(Feature::Llm)
        );
        for tag in [
            TAG_SIGN_OUT,
            TAG_CANCEL_JOIN,
            TAG_PAUSE,
            TAG_CLEAR_INPUT_LOG,
            0,
            5,
            -20,
        ] {
            assert_eq!(toggled_feature(tag), None);
        }
    }

    #[test]
    fn signed_out_menu_offers_join_and_create() {
        let lines = build_lines(
            &Display::SignedOut,
            &account(false),
            None,
            &History::default(),
        );
        assert_eq!(lines[0], Line::Text("未登录".to_owned()));
        assert!(lines.contains(&Line::Action(
            "输入匹配码加入…".to_owned(),
            TAG_JOIN_WITH_CODE
        )));
        assert!(lines.contains(&Line::Action("开通素笺云".to_owned(), TAG_CREATE_SPACE)));
        assert!(
            !lines.iter().any(
                |line| matches!(line, Line::Action(_, tag) if toggled_feature(*tag).is_some())
            )
        );
        assert!(!lines.contains(&Line::Action("暂停同步".to_owned(), TAG_PAUSE)));
    }

    #[test]
    fn signed_in_menu_shows_switch_states() {
        let lines = build_lines(
            &Display::SignedIn,
            &account(true),
            None,
            &History::default(),
        );
        assert!(lines.contains(&Line::Action(
            "同步学习数据与设置：开".to_owned(),
            toggle_tag(Feature::Sync)
        )));
        assert!(lines.contains(&Line::Action(
            "跨设备剪贴板：关".to_owned(),
            toggle_tag(Feature::Clipboard)
        )));
        assert!(lines.contains(&Line::Action(
            "退出登录（解绑这台 Mac）".to_owned(),
            TAG_SIGN_OUT
        )));
        // 剪贴板关着：不列历史
        assert!(!lines.contains(&Line::Text("还没有剪贴板记录".to_owned())));
    }

    #[test]
    fn signing_in_menu_can_cancel() {
        let mut menu = account(false);
        menu.signing_in = true;
        menu.note = Some("加入出错了".to_owned());
        let lines = build_lines(&Display::SignedOut, &menu, None, &History::default());
        assert!(lines.contains(&Line::Text("加入出错了".to_owned())));
        assert!(lines.contains(&Line::Action("取消".to_owned(), TAG_CANCEL_JOIN)));
        assert!(!lines.contains(&Line::Action(
            "输入匹配码加入…".to_owned(),
            TAG_JOIN_WITH_CODE
        )));
    }

    #[test]
    fn signed_in_menu_has_clear_input_log() {
        let lines = build_lines(
            &Display::SignedIn,
            &account(true),
            None,
            &History::default(),
        );
        assert!(lines.contains(&Line::Action(
            "清空云端输入记录…".to_owned(),
            TAG_CLEAR_INPUT_LOG
        )));
        assert!(lines.contains(&Line::Action(
            "云端记忆（把记下的素材整理成卡）：关".to_owned(),
            toggle_tag(Feature::Memory)
        )));
        assert!(lines.contains(&Line::Action(
            "同步打字内容：关".to_owned(),
            toggle_tag(Feature::InputLog)
        )));
    }

    fn data_status() -> DataStatus {
        DataStatus {
            last_ok_ms: Some(0),
            ..DataStatus::default()
        }
    }

    #[test]
    fn data_line_shows_expired_login() {
        let data = DataStatus {
            unauthorized: true,
            ..data_status()
        };
        let line = data_line(&data, account(true).consents).unwrap();
        assert_eq!(line, "学习数据：登录已失效");
    }

    #[test]
    fn data_line_hidden_when_every_enabled_item_is_stopped() {
        let consents = Consents {
            sync: true,
            input_log: true,
            ..Consents::default()
        };
        let mut data = DataStatus {
            disabled: vec![Feature::Sync],
            ..data_status()
        };
        // 还有输入日志在跑：照常显示，不报「刚刚同步」以外的东西
        assert!(data_line(&data, consents).is_some());
        data.disabled.push(Feature::InputLog);
        assert_eq!(data_line(&data, consents), None);
        // 只开了同步且被停：没有可显示的
        let only_sync = Consents {
            sync: true,
            ..Consents::default()
        };
        let stopped = DataStatus {
            disabled: vec![Feature::Sync],
            ..data_status()
        };
        let lines = build_lines(
            &Display::SignedIn,
            &AccountMenu {
                consents: only_sync,
                ..account(true)
            },
            Some(&stopped),
            &History::default(),
        );
        assert!(
            !lines
                .iter()
                .any(|line| matches!(line, Line::Text(text) if text.starts_with("学习数据")))
        );
        assert!(!lines.contains(&Line::Action("立即同步学习数据".to_owned(), TAG_SYNC_NOW)));
    }

    #[test]
    fn status_line_for_expired_and_disabled_clipboard() {
        let expired = Display::Sync {
            status: Status::Unauthorized,
            pending: 0,
        };
        assert_eq!(status_line(&expired), "登录已失效，请重新登录");
        let disabled = Display::Sync {
            status: Status::Disabled,
            pending: 2,
        };
        assert_eq!(
            status_line(&disabled),
            "跨设备剪贴板在服务器上没开 · 2 条待上传"
        );
    }

    #[test]
    fn unconfigured_reason_is_truncated() {
        let reason = "配置文件第 12 行格式不对：".to_owned() + &"很长".repeat(50);
        let line = status_line(&Display::Unconfigured(reason));
        assert_eq!(line.chars().count(), 40);
    }
}
