//! 账号：网页登录、用一次性码换令牌、退出登录、切换功能开关、跟着服务器的状态改本机开关。
//! 网络请求都在一次性的后台线程里，结果经通道回到主线程拍子（[`Service::apply_account_events`]）。
//! 剪贴板被服务器拒绝（403）时离线队列由 `ClipboardSync` 自己清，这里不管。

use std::path::Path;

use objc2::MainThreadMarker;
use qingjian_cloud_client::{Client, ClientError, ClipboardSync, DataSync, Status};
use qingjian_cloud_proto::{Consents, Device, Feature, HandoffExchange, Platform};

use super::Service;
use crate::account::{
    self, AccountEvent, WebLogin, progress_exists, reset_account_data, reset_after_sync_toggle,
    should_reset, sync_toggled,
};
use crate::config::AgentConfig;
use crate::paths;

impl Service {
    /// 「登录…」：生成 verifier，打开网页登录窗口。
    pub(super) fn sign_in(&mut self) {
        if self.login.is_some() || self.exchanging {
            return;
        }
        let (Some(config), Some(mtm)) = (&self.config, MainThreadMarker::new()) else {
            return;
        };
        let verifier = match account::new_verifier() {
            Ok(verifier) => verifier,
            Err(reason) => {
                self.note = Some(reason);
                self.refresh_menu(true);
                return;
            }
        };
        let device = account::device_name();
        let url = account::login_url(&config.server(), &account::challenge(&verifier), &device);
        match WebLogin::start(mtm, &url, verifier, device, self.sender.clone()) {
            Ok(login) => {
                tracing::info!("打开网页登录");
                self.login = Some(login);
                self.note = None;
            }
            Err(reason) => self.note = Some(reason),
        }
        self.refresh_menu(true);
    }

    /// 「取消登录」：丢掉登录窗口（取消会话、关窗），不再等换令牌的结果。
    pub(super) fn cancel_sign_in(&mut self) {
        self.login = None;
        self.exchanging = false;
        self.refresh_menu(true);
    }

    /// 「退出登录」：本机立即退出；服务器上的注销在后台发，失败只记日志。
    /// 同一账号再登录要保留同步进度与输入日志，所以这里不清它们。
    pub(super) fn sign_out(&mut self) {
        let Some(config) = self.config.as_ref().filter(|config| config.signed_in()) else {
            return;
        };
        let client = Client::new(&config.server(), &config.token);
        spawn("cloud-sign-out", move || {
            if let Err(error) = client.sign_out() {
                tracing::warn!(%error, "服务器上退出登录失败，本机已退出");
            }
        });
        self.note = None;
        self.store(AgentConfig::clear_session);
        tracing::info!("青简 Cloud 已退出登录");
        self.refresh_menu(true);
    }

    /// 切换一项功能：先告诉服务器，按服务器回的四项开关改本机。
    pub(super) fn toggle(&mut self, feature: Feature) {
        let Some(config) = self.config.as_ref().filter(|config| config.signed_in()) else {
            return;
        };
        let enabled = !config.consents().get(feature);
        let client = Client::new(&config.server(), &config.token);
        let sender = self.sender.clone();
        self.note = None;
        spawn("cloud-consent", move || {
            let event = match client.put_consent(feature, enabled) {
                Ok(consents) => AccountEvent::Consents(consents),
                Err(ClientError::Forbidden(_)) => AccountEvent::Forbidden(feature),
                Err(ClientError::Unauthorized) => AccountEvent::SignedOut,
                Err(error) => {
                    AccountEvent::Failed(format!("开关没改成：{}", account::reason(&error)))
                }
            };
            let _ = sender.send(event);
        });
    }

    /// 启动与「重新加载配置」时问一次服务器上的开关：别的设备改过的跟过来。
    pub(super) fn refresh_consents(&self) {
        let Some(config) = self.config.as_ref().filter(|config| config.signed_in()) else {
            return;
        };
        let client = Client::new(&config.server(), &config.token);
        let sender = self.sender.clone();
        spawn("cloud-account", move || {
            let event = match client.account() {
                Ok(account) => AccountEvent::Consents(account.consents),
                Err(ClientError::Unauthorized) => AccountEvent::SignedOut,
                Err(error) => {
                    tracing::info!(%error, "取账号失败，开关照本机配置");
                    return;
                }
            };
            let _ = sender.send(event);
        });
    }

    pub(super) fn apply_account_events(&mut self) {
        let mut changed = false;
        while let Ok(event) = self.events.try_recv() {
            self.apply(event);
            changed = true;
        }
        if changed {
            self.refresh_menu(true);
        }
    }

    /// 同步线程报 401 / 403：令牌失效就退出登录，某项功能服务器上没开就把本机开关记成关。
    pub(super) fn follow_server_state(&mut self) {
        let clipboard = self.sync.as_ref().map(ClipboardSync::status);
        let data = self.data.as_ref().map(DataSync::status);
        if clipboard == Some(Status::Unauthorized)
            || data.as_ref().is_some_and(|data| data.unauthorized)
        {
            self.apply(AccountEvent::SignedOut);
            self.refresh_menu(true);
            return;
        }
        let mut off = Vec::new();
        if clipboard == Some(Status::Disabled) {
            off.push(Feature::Clipboard);
        }
        if let Some(data) = &data {
            off.extend(data.disabled.iter().copied());
        }
        if !off.is_empty() {
            self.switch_off(&off);
            self.refresh_menu(true);
        }
    }

    fn apply(&mut self, event: AccountEvent) {
        match event {
            AccountEvent::Callback(Ok(url)) => self.exchange(&url),
            AccountEvent::Callback(Err(reason)) => {
                self.login = None;
                self.note = Some(reason);
            }
            AccountEvent::SignedIn {
                token,
                user_id,
                consents,
            } => {
                self.exchanging = false;
                self.note = None;
                self.signed_in_as(&token, user_id, consents);
            }
            // 退出登录后才到的旧结果不能再写进配置
            AccountEvent::Consents(_) | AccountEvent::Forbidden(_) if !self.signed_in() => {}
            AccountEvent::Consents(consents) => {
                if self.config.as_ref().map(AgentConfig::consents) != Some(consents) {
                    self.set_consents(consents);
                }
            }
            AccountEvent::Forbidden(feature) => {
                self.note = Some("这项功能现在不能打开".to_owned());
                self.switch_off(&[feature]);
            }
            AccountEvent::SignedOut => {
                self.login = None;
                self.exchanging = false;
                self.note = Some("登录已失效，请重新登录".to_owned());
                self.store(AgentConfig::clear_session);
            }
            AccountEvent::Failed(reason) => {
                self.exchanging = false;
                self.note = Some(reason);
            }
        }
    }

    /// 登录拿到令牌：换了账号（[`should_reset`]）才作废旧账号的同步进度与输入日志，同一账号重登保留。
    /// 先停同步线程、先清再写：清的时候新令牌还没落盘，不会带着它去传旧账号的数据。
    fn signed_in_as(&mut self, token: &str, user_id: i64, consents: Consents) {
        self.sync = None;
        self.data = None;
        if let (Some(support), Some(ime)) = (paths::support_dir(), paths::ime_dir()) {
            let previous = self.config.as_ref().and_then(|config| config.user_id);
            if should_reset(previous, user_id, progress_exists(&support)) {
                reset_account_data(&support, &ime);
            }
        }
        self.store(|path| AgentConfig::store_session(path, token, user_id, consents));
        tracing::info!("青简 Cloud 已登录");
    }

    /// 登录窗回跳：关掉窗口，在后台用一次性码与 verifier 换令牌，再问一次服务器上的开关。
    fn exchange(&mut self, url: &str) {
        let Some(login) = self.login.take() else {
            return;
        };
        let Some(handoff) = account::handoff_from_callback(url) else {
            self.note = Some("登录回调里没有一次性码，请重试".to_owned());
            return;
        };
        let Some(server) = self.config.as_ref().map(AgentConfig::server) else {
            return;
        };
        let request = HandoffExchange {
            handoff,
            verifier: login.verifier().to_owned(),
            device: Device {
                name: login.device().to_owned(),
                platform: Platform::Macos,
            },
        };
        drop(login);
        self.exchanging = true;
        let sender = self.sender.clone();
        spawn("cloud-login", move || {
            let event = match Client::anonymous(&server).exchange_handoff(&request) {
                Ok(grant) => {
                    let consents = Client::new(&server, &grant.token)
                        .account()
                        .map(|account| account.consents)
                        .inspect_err(|error| tracing::warn!(%error, "登录后取开关失败，先全关"))
                        .unwrap_or_default();
                    AccountEvent::SignedIn {
                        token: grant.token,
                        user_id: grant.user_id,
                        consents,
                    }
                }
                Err(error) => {
                    AccountEvent::Failed(format!("登录失败：{}", account::sign_in_reason(&error)))
                }
            };
            let _ = sender.send(event);
        });
    }

    /// 把这几项本机开关记成关。
    fn switch_off(&mut self, features: &[Feature]) {
        let Some(mut consents) = self.config.as_ref().map(AgentConfig::consents) else {
            return;
        };
        for &feature in features {
            consents.set(feature, false);
        }
        tracing::info!(?features, "服务器上没开，本机开关记成关");
        self.set_consents(consents);
    }

    /// 本机开关改成这一组并重起同步。「同步」的值变了（开或关）时先清学习数据与配置的同步进度：
    /// 服务器关同步会删云端数据，本机基线不清，重新打开只推增量、补不回去，配置还会走 409。值没变不清，免得白白重推全量。
    /// 清之前先停 `DataSync`，否则线程会把进度文件写回来（它的 Drop 不等线程退出，竞态窗口很小）。
    fn set_consents(&mut self, consents: Consents) {
        let was = self.config.as_ref().map(AgentConfig::consents);
        if let Some(was) = was
            && sync_toggled(Feature::Sync, was.sync, consents.sync)
        {
            self.data = None;
            if let Some(support) = paths::support_dir() {
                reset_after_sync_toggle(&support);
            }
        }
        self.store(|path| AgentConfig::store_consents(path, consents));
    }

    /// 改配置文件再按新配置重起同步（`DataSync` 的 `disabled` 只增不减，只有重建才恢复）；写不进去把原因显示在菜单里。
    fn store(&mut self, write: impl FnOnce(&Path) -> Result<(), String>) {
        match paths::config_path() {
            Some(path) => {
                if let Err(reason) = write(&path) {
                    tracing::warn!(%reason, "青简 Cloud 配置写不进去");
                    self.note = Some(reason);
                }
            }
            None => self.note = Some("找不到用户目录".to_owned()),
        }
        self.load_config();
    }
}

fn spawn(name: &str, work: impl FnOnce() + Send + 'static) {
    if let Err(error) = std::thread::Builder::new()
        .name(name.to_owned())
        .spawn(work)
    {
        tracing::warn!(%error, thread = name, "后台线程起不来");
    }
}
