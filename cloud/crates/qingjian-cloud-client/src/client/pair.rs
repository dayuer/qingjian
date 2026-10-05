//! 建空间与匹配码加设备：新设备输码申请、旧设备列表里允许，新设备轮询取令牌。
//!
//! 建空间、输码申请、轮询这三步发生在还没有令牌的新设备上，用 [`Client::anonymous`]；出码、
//! 列申请、允许 / 拒绝在旧设备上，要令牌。规格见 synon-ime `docs/superpowers/specs/2026-10-05-no-account-design.md`。

use qingjian_cloud_proto::{
    HEADER_PAIR_SECRET, PATH_PAIR_CODE, PATH_PAIR_JOIN, PATH_PAIR_REQUESTS, PATH_SPACE, PairCode,
    PairDecision, PairJoin, PairJoinGrant, PairPoll, PairRequestInfo, SessionGrant, SpaceCreate,
};

use super::reply::{Context, check};
use super::{Client, json};
use crate::ClientError;

impl Client {
    /// 建空间：开云服务的第一步，不要令牌。服务端建完直接给这台设备签会话，`new_user` 为真。
    pub fn create_space(&self, request: &SpaceCreate) -> Result<SessionGrant, ClientError> {
        let result = self
            .agent
            .post(self.url(PATH_SPACE))
            .config()
            .http_status_as_error(false)
            .build()
            .send_json(request);
        let mut response = check(result, Context::Login)?;
        json(response.body_mut().read_json())
    }

    /// 出一张匹配码给新设备输（要令牌）。
    pub fn pair_code(&self) -> Result<PairCode, ClientError> {
        let result = self
            .agent
            .post(self.url(PATH_PAIR_CODE))
            .config()
            .http_status_as_error(false)
            .build()
            .header("Authorization", self.bearer())
            .send_empty();
        let mut response = check(result, Context::Account)?;
        json(response.body_mut().read_json())
    }

    /// 新设备输码申请加入，不要令牌；返回的 `secret` 轮询时走请求头。码不对是 [`ClientError::BadCode`]，
    /// 空间满 5 台是 [`ClientError::DeviceLimit`]（满员时码不消费，还能给别人用）。
    pub fn pair_join(&self, request: &PairJoin) -> Result<PairJoinGrant, ClientError> {
        let result = self
            .agent
            .post(self.url(PATH_PAIR_JOIN))
            .config()
            .http_status_as_error(false)
            .build()
            .send_json(request);
        let mut response = check(result, Context::Login)?;
        json(response.body_mut().read_json())
    }

    /// 轮询这次申请的结果，不要令牌。允许后第一次取会带会话；再取就是 404（[`ClientError::Rejected`]），
    /// 那是「这次申请没了」，不是码不对。
    pub fn pair_poll(&self, request_id: &str, secret: &str) -> Result<PairPoll, ClientError> {
        let result = self
            .agent
            .get(self.url(&format!("{PATH_PAIR_JOIN}/{request_id}")))
            .config()
            .http_status_as_error(false)
            .build()
            .header(HEADER_PAIR_SECRET, secret)
            .call();
        let mut response = check(result, Context::Login)?;
        json(response.body_mut().read_json())
    }

    /// 等这台设备处理的加入申请（要令牌）。
    pub fn pair_requests(&self) -> Result<Vec<PairRequestInfo>, ClientError> {
        let result = self
            .agent
            .get(self.url(PATH_PAIR_REQUESTS))
            .config()
            .http_status_as_error(false)
            .build()
            .header("Authorization", self.bearer())
            .call();
        let mut response = check(result, Context::Account)?;
        json(response.body_mut().read_json())
    }

    /// 允许或拒绝一条加入申请（要令牌）。允许之后新设备才取得到令牌。
    pub fn pair_decide(&self, request_id: &str, decision: PairDecision) -> Result<(), ClientError> {
        let result = self
            .agent
            .post(self.url(&format!("{PATH_PAIR_REQUESTS}/{request_id}")))
            .config()
            .http_status_as_error(false)
            .build()
            .header("Authorization", self.bearer())
            .send_json(decision);
        check(result, Context::Account)?;
        Ok(())
    }
}
