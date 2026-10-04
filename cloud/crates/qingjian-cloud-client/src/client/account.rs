//! 账号相关的接口：登录（不带令牌）、账号信息、功能开关、注销设备、退出登录、删账号。

use qingjian_cloud_proto::{
    Account, AppleSignIn, Consents, EmailStart, EmailVerify, Feature, HandoffExchange,
    PATH_ACCOUNT, PATH_AUTH_APPLE, PATH_AUTH_EMAIL_START, PATH_AUTH_EMAIL_VERIFY,
    PATH_AUTH_HANDOFF, PATH_CONSENTS, PATH_SESSIONS, PutConsent, SessionGrant,
};

use super::{Client, json};
use crate::ClientError;

impl Client {
    /// 登录接口不要令牌。
    pub fn anonymous(server: &str) -> Self {
        Self::new(server, "")
    }

    /// Apple 登录，不带 `Authorization`。请求里不能带 `challenge`：带了服务端回 `HandoffGrant`，
    /// 这个方法会报 `BadResponse`。
    pub fn sign_in_apple(&self, request: &AppleSignIn) -> Result<SessionGrant, ClientError> {
        let mut response = self
            .agent
            .post(self.url(PATH_AUTH_APPLE))
            .send_json(request)?;
        json(response.body_mut().read_json())
    }

    /// 给邮箱发 6 位验证码；成功是 204，没有响应体。
    pub fn email_start(&self, email: &str) -> Result<(), ClientError> {
        let request = EmailStart {
            email: email.to_owned(),
        };
        self.agent
            .post(self.url(PATH_AUTH_EMAIL_START))
            .send_json(&request)?;
        Ok(())
    }

    /// 校验邮箱验证码，成功换到会话。
    pub fn email_verify(&self, request: &EmailVerify) -> Result<SessionGrant, ClientError> {
        let mut response = self
            .agent
            .post(self.url(PATH_AUTH_EMAIL_VERIFY))
            .send_json(request)?;
        json(response.body_mut().read_json())
    }

    /// 网页登录回跳的一次性码加 verifier 换会话。
    pub fn exchange_handoff(&self, request: &HandoffExchange) -> Result<SessionGrant, ClientError> {
        let mut response = self
            .agent
            .post(self.url(PATH_AUTH_HANDOFF))
            .send_json(request)?;
        json(response.body_mut().read_json())
    }

    /// 当前令牌对应的账号信息与四项开关。
    pub fn account(&self) -> Result<Account, ClientError> {
        let mut response = self
            .agent
            .get(self.url(PATH_ACCOUNT))
            .header("Authorization", self.bearer())
            .call()?;
        json(response.body_mut().read_json())
    }

    /// 开关一项功能，返回服务器上新的四项开关。关掉时服务器删掉这部分云端数据。
    pub fn put_consent(&self, feature: Feature, enabled: bool) -> Result<Consents, ClientError> {
        let mut response = self
            .agent
            .put(self.url(&format!("{PATH_CONSENTS}/{}", feature.as_str())))
            .header("Authorization", self.bearer())
            .send_json(PutConsent { enabled })?;
        json(response.body_mut().read_json())
    }

    /// 注销本账号的某台设备。
    pub fn revoke_session(&self, id: i64) -> Result<(), ClientError> {
        self.agent
            .delete(self.url(&format!("{PATH_SESSIONS}/{id}")))
            .header("Authorization", self.bearer())
            .call()?;
        Ok(())
    }

    /// 退出登录：注销发请求的这台设备。
    pub fn sign_out(&self) -> Result<(), ClientError> {
        self.agent
            .delete(self.url(&format!("{PATH_SESSIONS}/current")))
            .header("Authorization", self.bearer())
            .call()?;
        Ok(())
    }

    /// 删账号：服务器删掉这个账号的全部数据，所有设备的会话立即失效。
    pub fn delete_account(&self) -> Result<(), ClientError> {
        self.agent
            .delete(self.url(PATH_ACCOUNT))
            .header("Authorization", self.bearer())
            .call()?;
        Ok(())
    }
}
