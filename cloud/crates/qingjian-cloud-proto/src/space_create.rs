//! `POST /v1/space` 的请求体：不要任何身份，直接建一个新空间并给这台设备签会话，响应是 `SessionGrant`（`new_user` 为真）。

use serde::{Deserialize, Serialize};

use crate::Device;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpaceCreate {
    pub device: Device,

    /// 同意把数据发到境外服务器的文本版本号，缺省空串；服务端缺字段、空串、未知版本都返回 400 `consent_required`。
    #[serde(default)]
    pub cross_border_consent: String,
}
