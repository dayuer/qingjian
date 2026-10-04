//! 菜单内容：连接状态、学习数据状态、最近的剪贴板（点一条复制到本机）、暂停与几个操作。
//! 只算出行列表，由输入法画成「中☁ → 青简 Cloud ›」子菜单。
//! tag 区分动作：非负数是历史条目的下标，负数是固定动作（输入法转发 -100..100 的 tag）。

mod display;
mod lines;

use qingjian_cloud_client::{DataStatus, Status};

use crate::history::History;

pub use display::Display;
pub use lines::Line;

pub const TAG_PAUSE: isize = -1;
pub const TAG_RELOAD: isize = -2;
pub const TAG_OPEN_CONFIG: isize = -3;
pub const TAG_SYNC_NOW: isize = -6;

pub fn build_lines(display: &Display, data: Option<&DataStatus>, history: &History) -> Vec<Line> {
    let mut lines = vec![Line::Text(status_line(display))];
    if let Some(data) = data {
        lines.push(Line::Text(data_line(data)));
    }
    lines.push(Line::Separator);
    let before = lines.len();
    for (index, entry) in history.entries().enumerate() {
        lines.push(Line::Action(entry.title(), index as isize));
    }
    if lines.len() == before {
        lines.push(Line::Text("还没有剪贴板记录".to_owned()));
    }
    lines.push(Line::Separator);
    let pause = if matches!(display, Display::Paused) {
        "继续同步"
    } else {
        "暂停同步"
    };
    lines.push(Line::Action(pause.to_owned(), TAG_PAUSE));
    if data.is_some() {
        lines.push(Line::Action("立即同步学习数据".to_owned(), TAG_SYNC_NOW));
    }
    lines.push(Line::Action("重新加载配置".to_owned(), TAG_RELOAD));
    lines.push(Line::Action("打开配置文件…".to_owned(), TAG_OPEN_CONFIG));
    lines
}

pub fn status_line(display: &Display) -> String {
    match display {
        Display::Unconfigured(reason) => reason.clone(),
        Display::Paused => "已暂停同步".to_owned(),
        Display::Sync { status, pending } => {
            let base = match status {
                Status::Connecting => "正在连接…".to_owned(),
                Status::Online => "已连接".to_owned(),
                Status::Offline(error) => format!("离线，稍后自动重试（{}）", short(error)),
                Status::Unauthorized => "令牌无效：在服务器上重新登记设备".to_owned(),
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

fn data_line(data: &DataStatus) -> String {
    let mut line = if let Some(error) = &data.error {
        format!("学习数据：同步失败，稍后重试（{}）", short(error))
    } else if data.waiting_for_ime {
        "学习数据：等输入法合并（切到青简打几个字）".to_owned()
    } else if let Some(ms) = data.last_ok_ms {
        format!("学习数据：{}同步", ago(ms))
    } else {
        "学习数据：正在同步…".to_owned()
    };
    if data.config_conflict {
        line.push_str(" · 设置有冲突，旧的一份已备份");
    }
    line
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
