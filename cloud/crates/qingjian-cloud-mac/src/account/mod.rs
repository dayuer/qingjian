//! 账号的可移植部分：设备名、给用户看的错误。开通与加入走 1b 的「没有账号」设计
//! （建空间 / 匹配码配对），见 `cloud/docs/design.md`。

mod event;
mod flow;
mod reset;

use qingjian_cloud_client::ClientError;

pub use self::event::AccountEvent;
pub use self::flow::AccountFlow;
pub use self::reset::{
    clear_input_log_files, progress_exists, request_input_log_reset, reset_account_data,
    reset_after_sync_toggle, should_reset, sync_toggled, take_input_log_reset,
};

/// 设备名取不到时报给服务端的名字。
const FALLBACK_DEVICE: &str = "Mac";

/// 设备名最长多少个字符。
const MAX_DEVICE_CHARS: usize = 64;

/// 当天验证码输错太多次：与 iOS 桥同一句。
const LOCKED_TODAY: &str = "今天验证失败次数过多，请明天再试，或改用 Apple 登录";

/// 「系统设置 → 通用 → 关于本机」里的电脑名，设备列表里显示。
pub fn device_name() -> String {
    let output = std::process::Command::new("scutil")
        .args(["--get", "ComputerName"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok());
    clean_device_name(output)
}

fn clean_device_name(raw: Option<String>) -> String {
    raw.map(|name| {
        name.chars()
            .filter(|c| !c.is_control())
            .collect::<String>()
            .trim()
            .chars()
            .take(MAX_DEVICE_CHARS)
            .collect::<String>()
    })
    .filter(|name| !name.is_empty())
    .unwrap_or_else(|| FALLBACK_DEVICE.to_owned())
}

/// 给用户看的失败原因，菜单里显示。403 的服务端原文是英文，只记日志不显示。
pub fn reason(error: &ClientError) -> String {
    match error {
        ClientError::Unreachable(_) | ClientError::Io(_) => {
            "连不上服务器，检查网络后再试".to_owned()
        }
        ClientError::Unauthorized => "登录已失效，请重新登录".to_owned(),
        ClientError::AuthFailed(_) => "验证没有通过，请重试".to_owned(),
        ClientError::NotConfigured(_) => "服务器暂时不支持这种登录方式".to_owned(),
        ClientError::Forbidden(reason) => {
            tracing::info!(%reason, "服务器拒绝了这项功能");
            "这项功能还没打开".to_owned()
        }
        ClientError::LockedToday(_) => LOCKED_TODAY.to_owned(),
        // Mac 只走网页登录，不会发出这个请求；兜底文案
        ClientError::ConsentRequired(_) => "需要先同意把数据发到境外服务器".to_owned(),
        ClientError::RateLimited => "操作太频繁，请稍后再试".to_owned(),
        // Mac 这一版只出码、不输码，匹配码与满员只会出现在新设备那一侧；兜底文案
        ClientError::BadCode(_) => "匹配码不对或已经过期".to_owned(),
        ClientError::DeviceLimit(_) => "空间里的设备已经满了".to_owned(),
        ClientError::Rejected { status, .. } => format!("服务器拒绝了请求（{status}）"),
        ClientError::BadResponse(_) => "服务器的回应看不懂，请升级输入法".to_owned(),
        // Mac 不做 2B 素材登记，只会从别的路径透传过来；兜底文案
        ClientError::ContactLimit => "云端的对象名单满了（100 个）".to_owned(),
    }
}

/// 开通 / 加入失败的原因：验证类错误有专门的话，其余同 [`reason`]。
pub fn join_failure_reason(error: &ClientError) -> String {
    match error {
        ClientError::AuthFailed(_) => "验证没有通过，请重试".to_owned(),
        other => reason(other),
    }
}

#[cfg(test)]
mod tests {
    use qingjian_cloud_client::ClientError;

    use super::*;

    #[test]
    fn device_name_falls_back_and_is_trimmed() {
        assert_eq!(clean_device_name(None), "Mac");
        assert_eq!(clean_device_name(Some("  \n".to_owned())), "Mac");
        assert_eq!(
            clean_device_name(Some("李的 MacBook\n".to_owned())),
            "李的 MacBook"
        );
        assert_eq!(
            clean_device_name(Some("长".repeat(100))).chars().count(),
            64
        );
        assert_eq!(
            clean_device_name(Some("A\u{7}\nB\u{1b}[0m".to_owned())),
            "AB[0m"
        );
        assert_eq!(clean_device_name(Some("\u{7}".to_owned())), "Mac");
    }

    #[test]
    fn join_failure_reasons_follow_the_error_variant() {
        assert_eq!(
            join_failure_reason(&ClientError::AuthFailed("x".to_owned())),
            "验证没有通过，请重试"
        );
        assert_eq!(
            join_failure_reason(&ClientError::BadCode("x".to_owned())),
            "匹配码不对或已经过期"
        );
        assert_eq!(
            join_failure_reason(&ClientError::DeviceLimit("x".to_owned())),
            "空间里的设备已经满了"
        );
        assert_eq!(
            join_failure_reason(&ClientError::Unreachable("x".to_owned())),
            "连不上服务器，检查网络后再试"
        );
    }

    #[test]
    fn general_reasons_never_show_server_text() {
        assert_eq!(reason(&ClientError::Unauthorized), "登录已失效，请重新登录");
        assert_eq!(
            reason(&ClientError::Forbidden("english".to_owned())),
            "这项功能还没打开"
        );
        assert_eq!(reason(&ClientError::RateLimited), "操作太频繁，请稍后再试");
        assert_eq!(
            reason(&ClientError::Rejected {
                status: 400,
                message: "english".to_owned()
            }),
            "服务器拒绝了请求（400）"
        );
        assert_eq!(
            reason(&ClientError::BadResponse("x".to_owned())),
            "服务器的回应看不懂，请升级输入法"
        );
    }
}
