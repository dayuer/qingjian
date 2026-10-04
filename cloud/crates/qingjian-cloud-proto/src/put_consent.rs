//! `PUT /v1/consents/{feature}` 的请求体；响应是新的 [`crate::Consents`]。关掉即删云端这部分数据。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PutConsent {
    pub enabled: bool,
}
