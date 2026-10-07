//! 菜单内容：连接状态、账号（登录 / 四个开关 / 退出登录）、学习数据状态、最近的剪贴板（点一条复制到本机）、暂停与几个操作。
//! 只算出行列表，由输入法画成「中☁ → 素笺云 ›」子菜单（输入法只认文字行与可点项，开关画成「名字：开 / 关」）。
//! tag 区分动作：非负数是历史条目的下标，负数是固定动作（输入法转发 -100..100 的 tag）。

mod account_menu;
mod display;
mod lines;

use qingjian_cloud_client::{DataStatus, Status};
use qingjian_cloud_proto::{Consents, Feature};

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

/// 「云服务设置…」：输入法壳拦截它，打开偏好设置的云服务页。
pub const TAG_OPEN_SETTINGS: isize = -18;

/// 菜单里只剩两行：一行状态、一个进设置页的入口。功能开关、暂停、清空这些全在
/// 偏好设置的「云服务」页里（菜单在 IMK 下又点不动又挤，2026-10-07 移过去）。
pub fn build_lines(
    display: &Display,
    _account: &AccountMenu,
    _data: Option<&DataStatus>,
) -> Vec<Line> {
    vec![
        Line::Text(status_line(display)),
        Line::Action("云服务设置…".to_owned(), TAG_OPEN_SETTINGS),
    ]
}

pub fn status_line(display: &Display) -> String {
    match display {
        Display::Unconfigured(reason) => short(reason),
        Display::SignedOut => "未开通素笺云".to_owned(),
        Display::SignedIn => "已开通".to_owned(),
        Display::Paused => "已暂停同步".to_owned(),
        Display::Sync { status, pending } => {
            let base = match status {
                Status::Connecting => "正在连接…".to_owned(),
                Status::Online => "已连接".to_owned(),
                Status::Offline(error) => format!("离线，稍后自动重试（{}）", short(error)),
                Status::Unauthorized => "授权已失效，请重新加入".to_owned(),
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

/// 学习数据那一行；开着的几项都被服务器停了时为 `None`（对应开关马上会被记成关）。
pub fn data_line_text(data: &DataStatus, consents: Consents) -> Option<String> {
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
        "学习数据：授权已失效".to_owned()
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
pub(crate) fn ago(ms: i64) -> String {
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
    use qingjian_cloud_proto::{Consents, Feature};

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
    fn menu_is_just_status_and_settings() {
        // 功能开关、暂停、清空都移到偏好设置的云服务页了，菜单只剩两行
        for display in [Display::SignedOut, Display::SignedIn, Display::Paused] {
            let lines = build_lines(&display, &account(true), None);
            assert_eq!(lines.len(), 2, "{display:?}");
            assert!(matches!(lines[0], Line::Text(_)));
            assert_eq!(
                lines[1],
                Line::Action("云服务设置…".to_owned(), TAG_OPEN_SETTINGS)
            );
        }
        // 未开通时状态行说「未开通素笺云」
        let lines = build_lines(&Display::SignedOut, &account(false), None);
        assert_eq!(lines[0], Line::Text("未开通素笺云".to_owned()));
        // 提示行（加入失败等）也不再进菜单
        let mut menu = account(true);
        menu.note = Some("加入失败".to_owned());
        let lines = build_lines(&Display::SignedIn, &menu, None);
        assert_eq!(lines.len(), 2);
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
        let line = data_line_text(&data, account(true).consents).unwrap();
        assert_eq!(line, "学习数据：授权已失效");
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
        assert!(data_line_text(&data, consents).is_some());
        data.disabled.push(Feature::InputLog);
        assert_eq!(data_line_text(&data, consents), None);
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
        assert_eq!(status_line(&expired), "授权已失效，请重新加入");
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
