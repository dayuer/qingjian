//! 账号相关的接口：登录（不带令牌）、账号信息、功能开关、注销设备、退出登录、删账号。
//! 这些请求关掉 ureq 的「状态码即错误」，由 `reply::check` 读出服务端的错误文案再映射。

use qingjian_cloud_proto::{
    Account, AppleSignIn, Consents, EmailStart, EmailVerify, Feature, HandoffExchange,
    PATH_ACCOUNT, PATH_AUTH_APPLE, PATH_AUTH_EMAIL_START, PATH_AUTH_EMAIL_VERIFY,
    PATH_AUTH_HANDOFF, PATH_CONSENTS, PATH_SESSIONS, PutConsent, SessionGrant,
};

use super::reply::{Context, check};
use super::{Client, json};
use crate::ClientError;

impl Client {
    /// 登录接口不要令牌。
    pub fn anonymous(server: &str) -> Self {
        Self::new(server, "")
    }

    /// Apple 登录，不带 `Authorization`。请求里不能带 `challenge`（网页登录页专用）：
    /// 带了服务端回 `HandoffGrant`，这个方法会报 `BadResponse`；Mac 不走它，走 [`Client::exchange_handoff`]。
    pub fn sign_in_apple(&self, request: &AppleSignIn) -> Result<SessionGrant, ClientError> {
        let result = self
            .agent
            .post(self.url(PATH_AUTH_APPLE))
            .config()
            .http_status_as_error(false)
            .build()
            .send_json(request);
        let mut response = check(result, Context::Login)?;
        json(response.body_mut().read_json())
    }

    /// 给邮箱发 6 位验证码；成功是 204，没有响应体。`cross_border_consent` 是用户同意的文本版本号
    /// （[`qingjian_cloud_proto::CROSS_BORDER_CONSENT_VERSION`]），没同意传空串，服务端回 400 `consent_required`。
    pub fn email_start(&self, email: &str, cross_border_consent: &str) -> Result<(), ClientError> {
        let request = EmailStart {
            email: email.to_owned(),
            cross_border_consent: cross_border_consent.to_owned(),
        };
        let result = self
            .agent
            .post(self.url(PATH_AUTH_EMAIL_START))
            .config()
            .http_status_as_error(false)
            .build()
            .send_json(&request);
        check(result, Context::Login)?;
        Ok(())
    }

    /// 校验邮箱验证码，成功换到会话。请求里不能带 `challenge`（网页登录页专用）：
    /// 带了服务端回 `HandoffGrant`，这个方法会报 `BadResponse`；Mac 不走它，走 [`Client::exchange_handoff`]。
    pub fn email_verify(&self, request: &EmailVerify) -> Result<SessionGrant, ClientError> {
        let result = self
            .agent
            .post(self.url(PATH_AUTH_EMAIL_VERIFY))
            .config()
            .http_status_as_error(false)
            .build()
            .send_json(request);
        let mut response = check(result, Context::Login)?;
        json(response.body_mut().read_json())
    }

    /// 网页登录回跳的一次性码加 verifier 换会话。
    pub fn exchange_handoff(&self, request: &HandoffExchange) -> Result<SessionGrant, ClientError> {
        let result = self
            .agent
            .post(self.url(PATH_AUTH_HANDOFF))
            .config()
            .http_status_as_error(false)
            .build()
            .send_json(request);
        let mut response = check(result, Context::Login)?;
        json(response.body_mut().read_json())
    }

    /// 当前令牌对应的账号信息与四项开关。
    pub fn account(&self) -> Result<Account, ClientError> {
        let result = self
            .agent
            .get(self.url(PATH_ACCOUNT))
            .config()
            .http_status_as_error(false)
            .build()
            .header("Authorization", self.bearer())
            .call();
        let mut response = check(result, Context::Account)?;
        json(response.body_mut().read_json())
    }

    /// 开关一项功能，返回服务器上新的四项开关。关掉时服务器删掉这部分云端数据。
    pub fn put_consent(&self, feature: Feature, enabled: bool) -> Result<Consents, ClientError> {
        let result = self
            .agent
            .put(self.url(&format!("{PATH_CONSENTS}/{}", feature.as_str())))
            .config()
            .http_status_as_error(false)
            .build()
            .header("Authorization", self.bearer())
            .send_json(PutConsent { enabled });
        let mut response = check(result, Context::Account)?;
        json(response.body_mut().read_json())
    }

    /// 注销本账号的某台设备。
    pub fn revoke_session(&self, id: i64) -> Result<(), ClientError> {
        self.delete_checked(&format!("{PATH_SESSIONS}/{id}"))
    }

    /// 退出登录：注销发请求的这台设备。
    pub fn sign_out(&self) -> Result<(), ClientError> {
        self.delete_checked(&format!("{PATH_SESSIONS}/current"))
    }

    /// 删账号：服务器删掉这个账号的全部数据，所有设备的会话立即失效。
    pub fn delete_account(&self) -> Result<(), ClientError> {
        self.delete_checked(PATH_ACCOUNT)
    }

    fn delete_checked(&self, path: &str) -> Result<(), ClientError> {
        let result = self
            .agent
            .delete(self.url(path))
            .config()
            .http_status_as_error(false)
            .build()
            .header("Authorization", self.bearer())
            .call();
        check(result, Context::Account)?;
        Ok(())
    }
}
