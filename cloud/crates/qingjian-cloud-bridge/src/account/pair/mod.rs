//! 「不要账号」那条路：开通云服务时建空间，新设备用匹配码申请加入，旧设备上允许之后才拿到会话。
//! 令牌与登录一样只在 `cloud.toml` 与这里之间流转，不交给 Swift（轮询到会话就地落盘，只回报状态）。
//! 建空间、输码申请、轮询发生在还没有令牌的新设备上（`Client::anonymous`），出码、列申请、允许要已登录。

use std::path::Path;

use qingjian_cloud_client::{Client, ClientError};
use qingjian_cloud_proto::{
    PairCode, PairDecision, PairJoin, PairJoinGrant, PairPoll, PairRequestInfo, SpaceCreate,
};

use super::failure::{Failure, message};
use super::{consent_version, device, finish, server, signed_in};

/// 建空间：新设备开云服务的第一步，不用登录。服务端签发的会话交给 [`finish`] 落盘。
pub fn create_space(
    path: &Path,
    device_name: &str,
    cross_border_consented: bool,
) -> Result<(), Failure> {
    let consent = consent_version(cross_border_consented)?;
    let server = server(path);
    let request = SpaceCreate {
        device: device(device_name),
        cross_border_consent: consent.to_owned(),
    };
    let grant = Client::anonymous(&server)
        .create_space(&request)
        .map_err(|error| Failure::from_client(&error, message(&error)))?;
    finish(path, &server, &grant)
}

/// 出一张匹配码给新设备输（要已登录）。
pub fn pair_code(path: &Path) -> Result<PairCode, Failure> {
    let config = signed_in(path)?;
    Client::new(&config.server_or_default(), &config.token)
        .pair_code()
        .map_err(|error| super::expired(path, &error))
}

/// 新设备输码申请加入，不要把会话交给调用方——轮询才是取令牌那一步。返回的 `secret` 是轮询的凭据。
pub fn pair_join(path: &Path, code: &str, device_name: &str) -> Result<PairJoinGrant, Failure> {
    let request = PairJoin {
        // 服务端会再规范化一遍；这里只去掉用户可能带上的空白
        code: code.trim().to_owned(),
        device: device(device_name),
    };
    Client::anonymous(&server(path))
        .pair_join(&request)
        .map_err(|error| Failure::from_client(&error, pair_message(&error)))
}

/// 轮询这次申请的结果。允许时服务端给的会话就地写进 `cloud.toml`，只回报状态。
pub fn pair_poll(path: &Path, request_id: &str, secret: &str) -> Result<PairPoll, Failure> {
    let server = server(path);
    let poll = Client::anonymous(&server)
        .pair_poll(request_id, secret)
        .map_err(|error| Failure::from_client(&error, pair_message(&error)))?;
    if let PairPoll::Approved(grant) = &poll {
        finish(path, &server, grant)?;
    }
    Ok(poll)
}

/// 等这台设备处理的加入申请（要已登录）。
pub fn pair_requests(path: &Path) -> Result<Vec<PairRequestInfo>, Failure> {
    let config = signed_in(path)?;
    Client::new(&config.server_or_default(), &config.token)
        .pair_requests()
        .map_err(|error| super::expired(path, &error))
}

/// 允许或拒绝一条加入申请（要已登录）。允许之后新设备才取得到令牌。
pub fn pair_decide(path: &Path, request_id: &str, allow: bool) -> Result<(), Failure> {
    let config = signed_in(path)?;
    Client::new(&config.server_or_default(), &config.token)
        .pair_decide(request_id, PairDecision { allow })
        .map_err(|error| super::expired(path, &error))
}

/// 匹配码这条路自己的说法：码不对要重输，空间满要先去旧设备删一台，其余同 [`message`]。
fn pair_message(error: &ClientError) -> String {
    match error {
        ClientError::BadCode(_) => "匹配码不对或已经过期，请重新输一张".to_owned(),
        ClientError::DeviceLimit(_) => "空间里的设备已经满了，先在旧设备上删一台再加".to_owned(),
        other => message(other),
    }
}

#[cfg(test)]
mod tests;
