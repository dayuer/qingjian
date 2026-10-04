//! 账号的可移植部分：PKCE 的 verifier / challenge、网页登录页地址、回跳地址里的一次性码、设备名、给用户看的错误。
//! 真正打开登录窗口的 [`WebLogin`] 与它的展示锚点只在 macOS 上编。

mod event;
mod reset;

#[cfg(target_os = "macos")]
mod anchor;
#[cfg(target_os = "macos")]
mod web_login;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};
use qingjian_cloud_client::ClientError;
use qingjian_cloud_proto::{LOGIN_CALLBACK_SCHEME, PATH_LOGIN};
use sha2::{Digest, Sha256};

pub use self::event::AccountEvent;
pub use self::reset::{
    progress_exists, reset_account_data, reset_after_sync_toggle, should_reset, sync_toggled,
};

#[cfg(target_os = "macos")]
pub use self::anchor::LoginAnchor;
#[cfg(target_os = "macos")]
pub use self::web_login::WebLogin;

/// 查询参数里要转义的字符：RFC 3986 的非保留字符之外都转。
const QUERY: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

/// 回跳地址的主机部分：`sujian://auth?handoff=…`。
const CALLBACK_HOST: &str = "auth";

/// 设备名取不到时报给服务端的名字。
const FALLBACK_DEVICE: &str = "Mac";

/// 设备名最长多少个字符。
const MAX_DEVICE_CHARS: usize = 64;

/// 当天验证码输错太多次：与 iOS 桥同一句。
const LOCKED_TODAY: &str = "今天验证失败次数过多，请明天再试，或改用 Apple 登录";

/// 32 字节随机数的 base64url（无填充，43 个字符），只留在本进程里。
pub fn new_verifier() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|error| format!("取不到随机数：{error}"))?;
    Ok(verifier_from(&bytes))
}

pub fn verifier_from(bytes: &[u8; 32]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

/// PKCE 的 S256：`base64url(SHA256(verifier))`，无填充。
pub fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

/// `{server}/login?challenge=…&device=…&callback=sujian`。
pub fn login_url(server: &str, challenge: &str, device: &str) -> String {
    format!(
        "{}{PATH_LOGIN}?challenge={}&device={}&callback={LOGIN_CALLBACK_SCHEME}",
        server.trim().trim_end_matches('/'),
        utf8_percent_encode(challenge, QUERY),
        utf8_percent_encode(device, QUERY),
    )
}

/// 从 `sujian://auth?handoff=…` 取出一次性码；别的地址、没有或为空返回 `None`。
pub fn handoff_from_callback(url: &str) -> Option<String> {
    let (scheme, rest) = url.split_once("://")?;
    if !scheme.eq_ignore_ascii_case(LOGIN_CALLBACK_SCHEME) {
        return None;
    }
    let (host, query) = rest.split_once('?')?;
    if !host
        .trim_end_matches('/')
        .eq_ignore_ascii_case(CALLBACK_HOST)
    {
        return None;
    }
    let query = query.split('#').next().unwrap_or_default();
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(key, _)| *key == "handoff")
        .and_then(|(_, value)| percent_decode_str(value).decode_utf8().ok())
        .map(|value| value.into_owned())
        .filter(|value| !value.is_empty())
}

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
        name.trim()
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
        ClientError::RateLimited => "操作太频繁，请稍后再试".to_owned(),
        ClientError::Rejected { status, .. } => format!("服务器拒绝了请求（{status}）"),
        ClientError::BadResponse(_) => "服务器的回应看不懂，请升级输入法".to_owned(),
    }
}

/// 用一次性码换令牌失败的原因：登录类 401 与 503 有专门的话，其余同 [`reason`]。
pub fn sign_in_reason(error: &ClientError) -> String {
    match error {
        ClientError::AuthFailed(_) => "登录没有通过验证，请重试".to_owned(),
        other => reason(other),
    }
}

#[cfg(test)]
mod tests {
    use qingjian_cloud_client::ClientError;

    use super::*;

    /// RFC 7636 附录 B 的例子。
    const RFC_VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";

    #[test]
    fn challenge_matches_rfc_7636() {
        assert_eq!(
            challenge(RFC_VERIFIER),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn verifier_is_43_chars_of_base64url() {
        let verifier = verifier_from(&[0xff; 32]);
        assert_eq!(verifier.len(), 43);
        assert!(
            verifier
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        );
        let random = new_verifier().unwrap();
        assert_eq!(random.len(), 43);
        assert_ne!(random, new_verifier().unwrap());
    }

    #[test]
    fn login_url_escapes_device_name() {
        let url = login_url("https://pinyin.synon.ai/", "abc-_", "李的 MacBook");
        assert_eq!(
            url,
            "https://pinyin.synon.ai/login?challenge=abc-_&device=%E6%9D%8E%E7%9A%84%20MacBook&callback=sujian"
        );
    }

    #[test]
    fn handoff_comes_from_the_sujian_callback_only() {
        assert_eq!(
            handoff_from_callback("sujian://auth?handoff=h%2B1&x=y").as_deref(),
            Some("h+1")
        );
        assert_eq!(
            handoff_from_callback("SUJIAN://auth/?x=1&handoff=abc#frag").as_deref(),
            Some("abc")
        );
        assert_eq!(handoff_from_callback("https://auth?handoff=abc"), None);
        assert_eq!(handoff_from_callback("sujian://other?handoff=abc"), None);
        assert_eq!(handoff_from_callback("sujian://auth?handoff="), None);
        assert_eq!(handoff_from_callback("sujian://auth"), None);
    }

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
    }

    #[test]
    fn sign_in_reasons_follow_the_error_variant() {
        assert_eq!(
            sign_in_reason(&ClientError::LockedToday("x".to_owned())),
            "今天验证失败次数过多，请明天再试，或改用 Apple 登录"
        );
        assert_eq!(
            sign_in_reason(&ClientError::AuthFailed("x".to_owned())),
            "登录没有通过验证，请重试"
        );
        assert_eq!(
            sign_in_reason(&ClientError::NotConfigured("x".to_owned())),
            "服务器暂时不支持这种登录方式"
        );
        assert_eq!(
            sign_in_reason(&ClientError::Unreachable("x".to_owned())),
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
