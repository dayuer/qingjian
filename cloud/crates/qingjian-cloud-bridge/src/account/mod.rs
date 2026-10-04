//! 主 App「账号」页背后的操作：登录（Apple、邮箱两步）、看账号、改功能开关、注销设备、退出登录、删账号。
//! 都是阻塞的网络请求，Swift 在后台调。会话令牌只在 `cloud.toml` 与这里之间流转，不交给 Swift。
//! 失败返回 [`Failure`]（code 加中文文案）。网络请求不在 `cloud.toml` 的写锁里做：先请求，拿到结果再读-改-写。

mod failure;
mod ffi;
mod reset;
mod status;

use std::path::Path;

use qingjian_cloud_client::{Client, ClientError};
use qingjian_cloud_proto::{
    AppleClient, AppleSignIn, Consents, Device, EmailVerify, Feature, Platform, SessionGrant,
};

use self::failure::{Failure, apple_message, email_start_message, email_verify_message};
use self::reset::{
    cloud_dir, reset_account_data, reset_after_sync_toggle, should_reset, sync_toggled,
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
) -> Result<(), Failure> {
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
        .map_err(|error| Failure::from_client(&error, apple_message(&error)))?;
    finish(path, &server, &grant)
}

pub fn email_start(path: &Path, email: &str) -> Result<(), Failure> {
    Client::anonymous(&server(path))
        .email_start(email.trim())
        .map_err(|error| Failure::from_client(&error, email_start_message(&error)))
}

pub fn email_verify(
    path: &Path,
    email: &str,
    code: &str,
    device_name: &str,
) -> Result<(), Failure> {
    let server = server(path);
    let request = EmailVerify {
        email: email.trim().to_owned(),
        code: code.trim().to_owned(),
        device: device(device_name),
        challenge: None,
    };
    let grant = Client::anonymous(&server)
        .email_verify(&request)
        .map_err(|error| Failure::from_client(&error, email_verify_message(&error)))?;
    finish(path, &server, &grant)
}

/// 先改服务器上的同意记录，成功后把服务器回的四项开关写回 `cloud.toml`。
/// 「同步」的值真的变了（开或关）时还要清掉本机学习数据与配置的同步进度：服务器关同步会删云端数据，
/// 本机基线不清，重新打开只推增量、补不回去。值没变（重复点、别的设备已改过）不清，免得白白重推全量。
/// 清进度失败只记日志，开关照样写回；已知竞态：键盘进程里正在跑的 `DataSync` 理论上可能在清除的同一刻把进度写回，
/// 窗口很小（App 会先释放旧引擎再开新的）；开关本身若与服务器不一致，下次 `AccountStatus::load` 以服务器为准写回。
pub fn set_consent(path: &Path, feature: Feature, enabled: bool) -> Result<(), Failure> {
    let config = signed_in(path)?;
    match client(&config).put_consent(feature, enabled) {
        Ok(consents) => {
            if sync_toggled(feature, config.sync, consents.sync) {
                reset_after_sync_toggle(path);
            }
            store(CloudConfig::store_consents(path, consents))
        }
        Err(error) => Err(expired(path, &error)),
    }
}

pub fn revoke_session(path: &Path, id: i64) -> Result<(), Failure> {
    let config = signed_in(path)?;
    client(&config)
        .revoke_session(id)
        .map_err(|error| expired(path, &error))
}

/// 本机总会退出（清掉令牌与开关）；服务器上没注销掉只记日志，可以从别的设备再注销这台。
/// 同一账号再登录要保留同步进度与输入日志，所以这里不清它们。
pub fn sign_out(path: &Path) -> Result<(), Failure> {
    if let Ok(config) = signed_in(path)
        && let Err(error) = client(&config).sign_out()
    {
        tracing::warn!(%error, "服务器上退出登录失败，本机照样退出");
    }
    store(CloudConfig::clear_session(path))
}

/// 服务器删成功才清本机：令牌、账号 id、同步进度与输入日志；令牌已经失效时也清令牌，免得界面一直显示已登录。
pub fn delete_account(path: &Path) -> Result<(), Failure> {
    let config = signed_in(path)?;
    match client(&config).delete_account() {
        Ok(()) => forget_account(path),
        Err(error) => Err(expired(path, &error)),
    }
}

/// 账号已在服务器上删掉：本机忘掉它，旧账号的数据不留给下一个账号。
fn forget_account(path: &Path) -> Result<(), Failure> {
    let result = store(CloudConfig::clear_account(path));
    reset_account_data(path);
    result
}

/// 登录拿到令牌：先问一次服务器上的开关（老用户在别的设备上开过的照旧开，取不到就全关），再写本机。
fn finish(path: &Path, server: &str, grant: &SessionGrant) -> Result<(), Failure> {
    let consents = Client::new(server, &grant.token)
        .account()
        .map(|account| account.consents)
        .inspect_err(|error| tracing::warn!(%error, "登录后取开关失败，先全关"))
        .unwrap_or_default();
    store_login(path, server, &grant.token, grant.user_id, consents)
}

/// 把登录结果写进 `cloud.toml`（地址、令牌、账号 id、开关）。换了账号（[`should_reset`]）才作废旧账号的数据，
/// 同一账号重登保留。先清再写：清的时候令牌还没落盘，键盘不会带着新令牌去传旧数据。
fn store_login(
    path: &Path,
    server: &str,
    token: &str,
    user_id: i64,
    consents: Consents,
) -> Result<(), Failure> {
    let previous = CloudConfig::read(path).and_then(|config| config.user_id);
    let cloud_dir_exists = cloud_dir(path).is_some_and(|dir| dir.exists());
    if should_reset(previous, user_id, cloud_dir_exists) {
        reset_account_data(path);
    }
    store(CloudConfig::store_session(
        path, server, token, user_id, consents,
    ))
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

/// 本机文件写不了：原因只记日志，给用户一句通用的话。
fn store(result: Result<(), String>) -> Result<(), Failure> {
    result.map_err(|reason| {
        tracing::warn!(%reason, "cloud.toml 写入失败");
        Failure::other("保存登录状态失败，请重试")
    })
}

fn server(path: &Path) -> String {
    CloudConfig::read(path)
        .unwrap_or_default()
        .server_or_default()
}

fn signed_in(path: &Path) -> Result<CloudConfig, Failure> {
    CloudConfig::read(path)
        .filter(CloudConfig::signed_in)
        .ok_or_else(Failure::not_signed_in)
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
fn expired(path: &Path, error: &ClientError) -> Failure {
    if matches!(error, ClientError::Unauthorized)
        && let Err(reason) = CloudConfig::clear_session(path)
    {
        tracing::warn!(%reason, "清令牌失败");
    }
    Failure::from_client(error, failure::message(error))
}

#[cfg(test)]
mod tests;
