//! 阻塞式 HTTP 客户端，一个方法对应一个接口。

use std::io::BufReader;
use std::time::Duration;

use qingjian_cloud_proto::{
    Event, EventPage, PATH_CLIPBOARD, PATH_EVENTS, PATH_STREAM, PATH_WHOAMI, PushClip, Whoami,
};
use ureq::Agent;
use ureq::config::ConfigBuilder;
use ureq::tls::{RootCerts, TlsConfig, TlsProvider};
use ureq::typestate::AgentScope;

use crate::{ClientError, SseReader};

/// 普通请求的总超时。
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// 一条 SSE 连接最长维持多久。ureq 没有按每次读取计时的空闲超时，读阻塞时发现不了死连接（合盖、换网络），
/// 所以给整个响应体一个上限，到时重连并按 `seq` 补拉；断线最多这么久就能发现。
const STREAM_BUDGET: Duration = Duration::from_secs(60);

#[derive(Clone)]
pub struct Client {
    /// 服务器地址，不带末尾的 `/`。
    base: String,

    token: String,

    agent: Agent,

    stream_agent: Agent,
}

impl Client {
    pub fn new(server: &str, token: &str) -> Self {
        let agent = with_tls(Agent::config_builder())
            .timeout_global(Some(REQUEST_TIMEOUT))
            .http_status_as_error(true)
            .build()
            .new_agent();
        let stream_agent = with_tls(Agent::config_builder())
            .timeout_connect(Some(REQUEST_TIMEOUT))
            .timeout_recv_response(Some(REQUEST_TIMEOUT))
            .timeout_recv_body(Some(STREAM_BUDGET))
            .http_status_as_error(true)
            .build()
            .new_agent();
        Self {
            base: server.trim().trim_end_matches('/').to_owned(),
            token: token.trim().to_owned(),
            agent,
            stream_agent,
        }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{path}", self.base)
    }

    fn bearer(&self) -> String {
        format!("Bearer {}", self.token)
    }

    pub fn whoami(&self) -> Result<Whoami, ClientError> {
        let mut response = self
            .agent
            .get(self.url(PATH_WHOAMI))
            .header("Authorization", self.bearer())
            .call()?;
        json(response.body_mut().read_json())
    }

    pub fn push_clip(&self, clip: &PushClip) -> Result<Event, ClientError> {
        let mut response = self
            .agent
            .post(self.url(PATH_CLIPBOARD))
            .header("Authorization", self.bearer())
            .send_json(clip)?;
        json(response.body_mut().read_json())
    }

    pub fn delete_clip(&self, seq: u64) -> Result<Event, ClientError> {
        let mut response = self
            .agent
            .delete(self.url(&format!("{PATH_CLIPBOARD}/{seq}")))
            .header("Authorization", self.bearer())
            .call()?;
        json(response.body_mut().read_json())
    }

    pub fn events(&self, since: u64, limit: usize) -> Result<EventPage, ClientError> {
        let mut response = self
            .agent
            .get(self.url(PATH_EVENTS))
            .header("Authorization", self.bearer())
            .query("since", since.to_string())
            .query("limit", limit.to_string())
            .call()?;
        json(response.body_mut().read_json())
    }

    /// 打开 SSE：先收 `since` 之后的积压，再收实时事件；最长 [`STREAM_BUDGET`] 后迭代器以错误结束。
    pub fn stream(&self, since: u64) -> Result<SseReader<impl std::io::BufRead>, ClientError> {
        let response = self
            .stream_agent
            .get(self.url(PATH_STREAM))
            .header("Authorization", self.bearer())
            .header("Accept", "text/event-stream")
            .query("since", since.to_string())
            .call()?;
        let reader = response.into_body().into_reader();
        Ok(SseReader::new(BufReader::new(reader)))
    }
}

/// 按编译时选的 TLS 后端配置；两个都开时用系统 TLS（信任用户在系统里装的证书）。
fn with_tls(builder: ConfigBuilder<AgentScope>) -> ConfigBuilder<AgentScope> {
    #[cfg(feature = "native-tls")]
    let tls = TlsConfig::builder()
        .provider(TlsProvider::NativeTls)
        .root_certs(RootCerts::PlatformVerifier)
        .build();
    #[cfg(not(feature = "native-tls"))]
    let tls = TlsConfig::builder()
        .provider(TlsProvider::Rustls)
        .root_certs(RootCerts::WebPki)
        .build();
    builder.tls_config(tls)
}

fn json<T>(result: Result<T, ureq::Error>) -> Result<T, ClientError> {
    result.map_err(|error| match error {
        ureq::Error::Json(error) => ClientError::BadResponse(error.to_string()),
        other => other.into(),
    })
}
