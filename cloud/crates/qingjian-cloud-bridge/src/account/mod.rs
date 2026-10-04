//! 主 App「账号」页背后的操作：登录（Apple、邮箱两步）、看账号、改功能开关、注销设备、退出登录、删账号。
//! 都是阻塞的网络请求，Swift 在后台调。会话令牌只在 `cloud.toml` 与这里之间流转，不交给 Swift。

mod ffi;
mod status;

use std::path::Path;

use qingjian_cloud_client::{Client, ClientError};
use qingjian_cloud_proto::{
    AppleClient, AppleSignIn, Consents, Device, EmailVerify, Feature, Platform, SessionGrant,
};

use crate::cloud_config::CloudConfig;

pub use self::status::AccountStatus;

/// 设备名为空时报给服务端的名字。
const FALLBACK_DEVICE: &str = "iPhone";

pub fn sign_in_apple(
    path: &Path,
    identity_token: &str,
    authorization_code: &str,
    nonce: &str,
    device_name: &str,
) -> Result<(), String> {
    let server = server(path);
    let request = AppleSignIn {
        identity_token: identity_token.to_owned(),
        authorization_code: authorization_code.to_owned(),
        nonce: nonce.to_owned(),
        client: AppleClient::Ios,
        device: device(device_name),
        challenge: None,
    };
    let grant = Client::anonymous(&server)
        .sign_in_apple(&request)
        .map_err(|error| apple_message(&error))?;
    finish(path, &server, &grant)
}

pub fn email_start(path: &Path, email: &str) -> Result<(), String> {
    Client::anonymous(&server(path))
        .email_start(email.trim())
        .map_err(|error| email_start_message(&error))
}

pub fn email_verify(path: &Path, email: &str, code: &str, device_name: &str) -> Result<(), String> {
    let server = server(path);
    let request = EmailVerify {
        email: email.trim().to_owned(),
        code: code.trim().to_owned(),
        device: device(device_name),
        challenge: None,
    };
    let grant = Client::anonymous(&server)
        .email_verify(&request)
        .map_err(|error| email_verify_message(&error))?;
    finish(path, &server, &grant)
}

/// 先改服务器上的同意记录，成功后把服务器回的四项开关写回 `cloud.toml`。
/// 「同步」的值真的变了（开或关）时还要清掉本机学习数据与配置的同步进度：服务器关同步会删云端数据，
/// 本机基线不清，重新打开只推增量、补不回去。值没变（重复点、别的设备已改过）不清，免得白白重推全量。
/// 清进度失败只记日志，开关照样写回；已知竞态：键盘进程里正在跑的 `DataSync` 理论上可能在清除的同一刻把进度写回，
/// 窗口很小（App 会先释放旧引擎再开新的）；开关本身若与服务器不一致，下次 `AccountStatus::load` 以服务器为准写回。
pub fn set_consent(path: &Path, feature: Feature, enabled: bool) -> Result<(), String> {
    let config = signed_in(path)?;
    match client(&config).put_consent(feature, enabled) {
        Ok(consents) => {
            if sync_toggled(feature, config.sync, consents.sync) {
                reset_after_sync_toggle(path);
            }
            CloudConfig::store_consents(path, consents)
        }
        Err(error) => Err(expired(path, &error)),
    }
}

pub fn revoke_session(path: &Path, id: i64) -> Result<(), String> {
    let config = signed_in(path)?;
    client(&config)
        .revoke_session(id)
        .map_err(|error| expired(path, &error))
}

/// 本机总会退出（清掉令牌与开关）；服务器上没注销掉只记日志，可以从别的设备再注销这台。
pub fn sign_out(path: &Path) -> Result<(), String> {
    if let Ok(config) = signed_in(path)
        && let Err(error) = client(&config).sign_out()
    {
        tracing::warn!(%error, "服务器上退出登录失败，本机照样退出");
    }
    CloudConfig::clear_session(path)
}

/// 服务器删成功才清本机令牌；令牌已经失效时也清掉，免得界面一直显示已登录。
pub fn delete_account(path: &Path) -> Result<(), String> {
    let config = signed_in(path)?;
    match client(&config).delete_account() {
        Ok(()) => CloudConfig::clear_account(path),
        Err(error) => Err(expired(path, &error)),
    }
}

/// 登录拿到令牌：先问一次服务器上的开关（老用户在别的设备上开过的照旧开，取不到就全关），
/// 连同地址、令牌、账号 id 写进 `cloud.toml`。换了账号（[`should_reset`]）才作废旧的同步进度，同一账号重登保留。
/// 换账号时整个删 `cloud/`：`outbox.jsonl` 里是旧账号的离线事件，不删会传到新账号；
/// 而开关切换只删三个状态文件（[`reset_after_sync_toggle`]），outbox 属于剪贴板，不该删。
fn finish(path: &Path, server: &str, grant: &SessionGrant) -> Result<(), String> {
    let consents = Client::new(server, &grant.token)
        .account()
        .map(|account| account.consents)
        .inspect_err(|error| tracing::warn!(%error, "登录后取开关失败，先全关"))
        .unwrap_or_default();
    let previous = CloudConfig::read(path).and_then(|config| config.user_id);
    let cloud_dir_exists = cloud_dir(path).is_some_and(|dir| dir.exists());
    if should_reset(previous, grant.user_id, cloud_dir_exists) {
        reset_sync_state(path);
    }
    CloudConfig::store_session(path, server, &grant.token, grant.user_id, consents)
}

/// 服务器上的开关与本机 `cloud.toml` 不一样时以服务器为准写回，返回是否清过同步进度。
/// 「同步」的值变了（用户在别的设备上关了又开，服务端已删过云端学习数据）要先清进度再写回；其它开关变化不清。
/// 没登录或读不了文件时什么也不做。清进度失败只记日志。
fn apply_server_consents(path: &Path, consents: Consents) -> bool {
    let Some(config) = CloudConfig::read(path).filter(CloudConfig::signed_in) else {
        return false;
    };
    if config.consents() == consents {
        return false;
    }
    let reset = sync_toggled(Feature::Sync, config.sync, consents.sync);
    if reset {
        reset_after_sync_toggle(path);
    }
    if let Err(reason) = CloudConfig::store_consents(path, consents) {
        tracing::warn!(%reason, "开关写回 cloud.toml 失败");
    }
    reset
}

/// 登录后要不要作废旧的同步进度：账号 id 变了要；本机没记过账号（旧版本写的文件、删号后）但留着进度，按换了账号处理。
fn should_reset(previous: Option<i64>, now: i64, cloud_dir_exists: bool) -> bool {
    match previous {
        Some(previous) => previous != now,
        None => cloud_dir_exists,
    }
}

/// 改的是「同步」且服务器给的新值与本机原来的不同。
fn sync_toggled(feature: Feature, was: bool, now: bool) -> bool {
    feature == Feature::Sync && was != now
}

fn cloud_dir(path: &Path) -> Option<std::path::PathBuf> {
    path.parent().map(|dir| dir.join("cloud"))
}

/// 只清学习数据与配置的同步进度（剪贴板与输入日志的留着），文件在 `cloud.toml` 同目录的 `cloud/` 下。
fn reset_after_sync_toggle(path: &Path) {
    let Some(dir) = cloud_dir(path) else {
        return;
    };
    if let Err(error) = qingjian_cloud_client::reset_sync_progress(&dir) {
        tracing::warn!(%error, "同步进度删不掉");
    }
}

/// 键盘的同步进度（学习数据基线、剪贴板进度）在 `cloud.toml` 同目录的 `cloud/` 下
/// （开了完全访问时学习数据目录就是 App Group 目录）。基线留着的话，新账号服务器上是空的，
/// 算出来「别的设备的增量」是负的，会把本机学到的减掉。
fn reset_sync_state(path: &Path) {
    let Some(dir) = cloud_dir(path) else {
        return;
    };
    match std::fs::remove_dir_all(&dir) {
        Ok(()) => tracing::info!("换了账号，清掉旧的同步进度"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => tracing::warn!(%error, "旧的同步进度删不掉"),
    }
}

fn server(path: &Path) -> String {
    CloudConfig::read(path)
        .unwrap_or_default()
        .server_or_default()
}

fn signed_in(path: &Path) -> Result<CloudConfig, String> {
    CloudConfig::read(path)
        .filter(CloudConfig::signed_in)
        .ok_or_else(|| "还没有登录".to_owned())
}

fn client(config: &CloudConfig) -> Client {
    Client::new(&config.server_or_default(), &config.token)
}

fn device(name: &str) -> Device {
    let name = name.trim();
    Device {
        name: if name.is_empty() {
            FALLBACK_DEVICE.to_owned()
        } else {
            name.to_owned()
        },
        platform: Platform::Ios,
    }
}

/// 令牌失效（被别的设备注销、账号删了）：清掉本机令牌，界面回到未登录。
fn expired(path: &Path, error: &ClientError) -> String {
    if matches!(error, ClientError::Unauthorized) {
        if let Err(reason) = CloudConfig::clear_session(path) {
            tracing::warn!(%reason, "清令牌失败");
        }
        return "登录已失效，请重新登录".to_owned();
    }
    message(error)
}

/// 当天验证码输错太多次：email/start 与 email/verify 都被锁到明天。
const LOCKED_TODAY: &str = "今天验证失败次数过多，请明天再试，或改用 Apple 登录";

fn apple_message(error: &ClientError) -> String {
    match error {
        ClientError::AuthFailed(_) => "Apple 登录没有通过验证，请重试".to_owned(),
        ClientError::NotConfigured(_) => "服务器还没配好 Apple 登录".to_owned(),
        other => message(other),
    }
}

fn email_start_message(error: &ClientError) -> String {
    match error {
        ClientError::Rejected {
            status: 400 | 422, ..
        } => "邮箱地址不对，检查后再试".to_owned(),
        ClientError::NotConfigured(_) => "服务器还没配好邮件发送".to_owned(),
        other => message(other),
    }
}

/// 验证码输错不显示剩余次数。
fn email_verify_message(error: &ClientError) -> String {
    match error {
        ClientError::AuthFailed(_) => "验证码不对，请重新输入".to_owned(),
        ClientError::NotConfigured(_) => "服务器还没配好邮件发送".to_owned(),
        other => message(other),
    }
}

/// 给用户看的失败原因。
fn message(error: &ClientError) -> String {
    match error {
        ClientError::Unreachable(_) | ClientError::Io(_) => {
            "连不上服务器，检查网络后再试".to_owned()
        }
        ClientError::Unauthorized => "登录已失效，请重新登录".to_owned(),
        ClientError::AuthFailed(_) => "验证没有通过，请重试".to_owned(),
        ClientError::NotConfigured(_) => "服务器暂时不支持这种登录方式".to_owned(),
        ClientError::Forbidden(reason) if !reason.trim().is_empty() => reason.trim().to_owned(),
        ClientError::Forbidden(_) => "服务器不允许这个操作".to_owned(),
        ClientError::LockedToday(_) => LOCKED_TODAY.to_owned(),
        ClientError::RateLimited => "操作太频繁，请稍后再试".to_owned(),
        ClientError::Rejected { status, .. } => format!("服务器拒绝了请求（{status}）"),
        ClientError::BadResponse(_) => "服务器的回应看不懂，请升级 App".to_owned(),
    }
}

#[cfg(test)]
mod tests;
