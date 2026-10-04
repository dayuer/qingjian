//! `GET /v1/account` 的响应：登录方式、设备与功能开关。

use serde::{Deserialize, Serialize};

use crate::{Consents, IdentityInfo, SessionInfo};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub identities: Vec<IdentityInfo>,

    pub sessions: Vec<SessionInfo>,

    pub consents: Consents,
}
