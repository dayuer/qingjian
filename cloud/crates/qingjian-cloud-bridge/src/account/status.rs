//! 账号页显示的内容：连哪台服务器、登没登录、四个开关、登录方式与设备。令牌不在里面。

use std::path::Path;

use qingjian_cloud_client::{Client, ClientError};
use qingjian_cloud_proto::{Consents, IdentityInfo, SessionInfo};
use serde::Serialize;

use crate::cloud_config::CloudConfig;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AccountStatus {
    /// 服务器地址，只读显示。
    pub server: String,

    pub signed_in: bool,

    /// 服务器上的开关；没登录或这次取不到时照 `cloud.toml`。
    pub consents: Consents,

    pub identities: Vec<IdentityInfo>,

    pub sessions: Vec<SessionInfo>,

    /// 这次没能从服务器取到账号的原因（中文）。
    pub error: Option<String>,

    /// 同一个原因的种类，取值同 [`super::failure::Failure`] 的 `code`；没有错误时省略。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<&'static str>,
}

impl AccountStatus {
    /// 没登录时只读文件、不联网。登录了就问一次服务器，开关和本机文件不一样时以服务器为准写回
    /// （键盘按 `cloud.toml` 的修改时间重开会话，所以一样时不写）；令牌失效就清掉，显示未登录。
    pub fn load(path: &Path) -> Self {
        let config = CloudConfig::read(path).unwrap_or_default();
        let mut status = Self::offline(&config);
        if !status.signed_in {
            return status;
        }
        match Client::new(&status.server, &config.token).account() {
            Ok(account) => {
                super::apply_server_consents(path, account.consents);
                status.consents = account.consents;
                status.identities = account.identities;
                status.sessions = account.sessions;
            }
            Err(ClientError::Unauthorized) => {
                if let Err(reason) = CloudConfig::clear_session(path) {
                    tracing::warn!(%reason, "清令牌失败");
                }
                status = Self::offline(&CloudConfig::read(path).unwrap_or_default());
                status.error = Some(super::failure::message(&ClientError::Unauthorized));
                status.error_code = Some("unauthorized");
            }
            Err(error) => {
                status.error = Some(super::failure::message(&error));
                status.error_code = Some(super::failure::code_of(&error));
            }
        }
        status
    }

    /// 只看本机文件。
    pub fn offline(config: &CloudConfig) -> Self {
        Self {
            server: config.server_or_default(),
            signed_in: config.signed_in(),
            consents: config.consents(),
            identities: Vec::new(),
            sessions: Vec::new(),
            error: None,
            error_code: None,
        }
    }
}
