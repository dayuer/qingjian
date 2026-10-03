//! 鉴权提取器：`Authorization: Bearer <令牌>` 换成设备。

use axum::extract::FromRequestParts;
use axum::http::header::AUTHORIZATION;
use axum::http::request::Parts;

use super::AppState;
use crate::ServerError;
use crate::store::Device;

pub struct AuthDevice(pub Device);

impl FromRequestParts<AppState> for AuthDevice {
    type Rejection = ServerError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .ok_or(ServerError::Unauthorized)?;
        state.store.authenticate(token).map(AuthDevice)
    }
}
