//! 账号：开通（建空间）、用匹配码加入（等旧设备允许）、退出登录、切换功能开关、跟着服务器的状态改本机开关。
//! 网络请求都在一次性的后台线程里，结果经通道回到主线程拍子（[`Service::apply_account_events`]）。
//! 剪贴板被服务器拒绝（403）时离线队列由 `ClipboardSync` 自己清，这里不管。

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use qingjian_cloud_client::{Client, ClientError, ClipboardSync, DataSync, Status};
use qingjian_cloud_proto::{
    CROSS_BORDER_CONSENT_VERSION, Consents, Device, Feature, PairJoin, PairJoinGrant, PairPoll,
    Platform, SessionGrant, SpaceCreate,
};

use super::Service;
use crate::account::{
    self, AccountEvent, progress_exists, request_input_log_reset, reset_account_data,
    reset_after_sync_toggle, should_reset, sync_toggled,
};
use crate::config::AgentConfig;
use crate::paths;

impl Service {
    /// 「开通素笺云」：建一个新空间，成功这台 Mac 就登录了。出境同意由壳弹窗确认后传进来。
    pub(super) fn create_space(&mut self, consented: bool) {
        if self.flow.signing_in() || self.signed_in() {
            return;
        }
        let Some(server) = self.config.as_ref().map(AgentConfig::server) else {
            self.note = Some(
                self.unconfigured
                    .clone()
                    .unwrap_or_else(|| "配置没读到，先点「重新加载配置」".to_owned()),
            );
            self.refresh_menu(true);
            return;
        };
        if !consented {
            self.note = Some("要先同意把数据发到境外服务器".to_owned());
            self.refresh_menu(true);
            return;
        }
        let device = account::device_name();
        let Some(generation) = self.flow.start_join() else {
            return;
        };
        let sender = self.sender.clone();
        self.note = None;
        self.refresh_menu(true);
        spawn("cloud-create-space", move || {
            let request = SpaceCreate {
                device: Device {
                    name: device,
                    platform: Platform::Macos,
                },
                cross_border_consent: CROSS_BORDER_CONSENT_VERSION.to_owned(),
            };
            let event = match Client::anonymous(&server).create_space(&request) {
                Ok(grant) => signed_in_event(&server, grant),
                Err(error) => AccountEvent::JoinFailed(format!(
                    "开通失败：{}",
                    account::join_failure_reason(&error)
                )),
            };
            let _ = sender.send((generation, event));
        });
    }

    /// 「输入匹配码加入…」：申请加入，成功后在这个线程里等旧设备（手机）上允许。
    /// `stop` 由 [`Self::cancel_join`] 置位，用户取消或退出时轮询立刻收手。
    pub(super) fn join_with_code(&mut self, code: &str) {
        if self.flow.signing_in() || self.signed_in() {
            return;
        }
        let Some(server) = self.config.as_ref().map(AgentConfig::server) else {
            self.note = Some("配置没读到，先点「重新加载配置」".to_owned());
            self.refresh_menu(true);
            return;
        };
        let device = account::device_name();
        let request = PairJoin {
            code: code.to_owned(),
            device: Device {
                name: device,
                platform: Platform::Macos,
            },
        };
        let Some(generation) = self.flow.start_join() else {
            return;
        };
        let stop = self.join_stop.clone();
        stop.store(false, Ordering::Release);
        let sender = self.sender.clone();
        self.note = None;
        self.refresh_menu(true);
        spawn("cloud-pair-join", move || {
            let client = Client::anonymous(&server);
            let grant = match client.pair_join(&request) {
                Ok(grant) => grant,
                Err(error) => {
                    let _ = sender.send((
                        generation,
                        AccountEvent::JoinFailed(format!(
                            "加入失败：{}",
                            account::join_failure_reason(&error)
                        )),
                    ));
                    return;
                }
            };
            let event = poll_until_decided(&server, &grant, &stop);
            let _ = sender.send((generation, event));
        });
    }

    /// 「清空云端输入记录…」：清服务端全部输入记录，同时清本机日志与上传进度。
    pub(super) fn clear_input_log(&mut self) {
        let Some(config) = self.config.as_ref().filter(|config| config.signed_in()) else {
            self.note = Some("先开通素笺云".to_owned());
            self.refresh_menu(true);
            return;
        };
        let client = Client::new(&config.server(), &config.token);
        let sender = self.sender.clone();
        let generation = self.flow.current();
        self.note = None;
        spawn("cloud-clear-input-log", move || {
            let event = match client.clear_input_log() {
                Ok(_) => {
                    if let (Some(support), Some(ime)) = (paths::support_dir(), paths::ime_dir()) {
                        account::clear_input_log_files(&support, &ime);
                    }
                    AccountEvent::Cleared
                }
                Err(error) => AccountEvent::Failed(format!(
                    "清空没成功：{}",
                    account::join_failure_reason(&error)
                )),
            };
            let _ = sender.send((generation, event));
        });
    }

    /// 「取消」：不再等旧设备允许，丢掉这次的结果。
    pub(super) fn cancel_join(&mut self) {
        self.join_stop.store(true, Ordering::Release);
        self.flow.cancel();
        self.note = None;
        self.refresh_menu(true);
    }

    /// 「退出登录」：本机立即退出；服务器上的注销在后台发，失败只记日志。
    /// 同一账号再登录要保留同步进度与输入日志，所以这里不清它们。
    pub(super) fn sign_out(&mut self) {
        let Some(config) = self.config.as_ref().filter(|config| config.signed_in()) else {
            return;
        };
        let client = Client::new(&config.server(), &config.token);
        // 旧令牌上还在路上的请求（开关、取账号）结果晚到不能再动新会话
        self.flow.invalidate();
        spawn("cloud-sign-out", move || {
            if let Err(error) = client.sign_out() {
                tracing::warn!(%error, "服务器上退出登录失败，本机已退出");
            }
        });
        self.note = None;
        self.store(AgentConfig::clear_session);
        tracing::info!("素笺云已退出登录");
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
        let generation = self.flow.current();
        self.note = None;
        spawn("cloud-consent", move || {
            let event = match client.put_consent(feature, enabled) {
                Ok(consents) => AccountEvent::Account {
                    consents,
                    sessions: None,
                },
                Err(ClientError::Forbidden(_)) => AccountEvent::Forbidden(feature),
                Err(ClientError::Unauthorized) => AccountEvent::SignedOut,
                Err(error) => {
                    AccountEvent::Failed(format!("开关没改成：{}", account::reason(&error)))
                }
            };
            let _ = sender.send((generation, event));
        });
    }

    /// 启动、打开云服务设置页与「重新加载配置」时问一次服务器：别的设备改过的开关跟过来，
    /// 顺便拿到同一空间里的设备列表。
    pub(super) fn refresh_account(&self) {
        let Some(config) = self.config.as_ref().filter(|config| config.signed_in()) else {
            return;
        };
        let client = Client::new(&config.server(), &config.token);
        let sender = self.sender.clone();
        let generation = self.flow.current();
        spawn("cloud-account", move || {
            let event = match client.account() {
                Ok(account) => AccountEvent::Account {
                    consents: account.consents,
                    sessions: Some(account.sessions),
                },
                Err(ClientError::Unauthorized) => AccountEvent::SignedOut,
                Err(error) => {
                    tracing::info!(%error, "取账号失败，开关照本机配置");
                    return;
                }
            };
            let _ = sender.send((generation, event));
        });
    }

    /// 解绑同一空间里的另一台设备（设置页设备列表里点的那一颗；壳已经弹过确认）。
    pub(super) fn revoke_device(&mut self, id: i64, name: String) {
        let Some(config) = self.config.as_ref().filter(|config| config.signed_in()) else {
            return;
        };
        let client = Client::new(&config.server(), &config.token);
        let sender = self.sender.clone();
        let generation = self.flow.current();
        spawn("cloud-account", move || {
            let event = match client.revoke_session(id) {
                Ok(()) => AccountEvent::Revoked(name),
                Err(ClientError::Unauthorized) => AccountEvent::SignedOut,
                Err(error) => {
                    AccountEvent::Failed(format!("解绑没成功：{}", account::reason(&error)))
                }
            };
            let _ = sender.send((generation, event));
        });
    }

    pub(super) fn apply_account_events(&mut self) {
        let mut changed = false;
        while let Ok((generation, event)) = self.events.try_recv() {
            // 取消、退出、重新登录之后才到的旧结果（旧令牌的 401 也算）直接丢掉
            if !self.flow.accepts(generation) {
                tracing::debug!(?event, "丢弃过期的账号事件");
                continue;
            }
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
            AccountEvent::JoinFailed(reason) => {
                self.flow.finish();
                self.note = Some(reason);
            }
            AccountEvent::SignedIn {
                token,
                user_id,
                consents,
            } => {
                self.flow.finish();
                self.flow.invalidate();
                self.note = None;
                self.signed_in_as(&token, user_id, consents);
            }
            // 退出登录后才到的结果不能再写进配置
            AccountEvent::Account { .. } | AccountEvent::Forbidden(_) if !self.signed_in() => {}
            AccountEvent::Account { consents, sessions } => {
                if let Some(sessions) = sessions
                    && self.sessions != sessions
                {
                    self.sessions = sessions;
                    // 设备列表变了：版本号一动，输入法的每拍会顺手刷开着的设置窗口
                    self.revision += 1;
                }
                if self.config.as_ref().map(AgentConfig::consents) != Some(consents) {
                    self.set_consents(consents);
                }
            }
            // 解绑成功：菜单给一句确认，顺手把设备列表取新的（那台已经不在里面了）
            AccountEvent::Revoked(name) => {
                self.note = Some(format!("已解绑「{name}」"));
                self.refresh_account();
            }
            AccountEvent::Forbidden(feature) => {
                self.note = Some("这项功能现在不能打开".to_owned());
                self.switch_off(&[feature]);
            }
            AccountEvent::SignedOut => {
                self.flow.cancel();
                self.sessions.clear();
                self.note = Some("登录已失效，请重新登录".to_owned());
                self.store(AgentConfig::clear_session);
            }
            AccountEvent::Failed(reason) => self.note = Some(reason),
            AccountEvent::Cleared => self.note = Some("已清空云端输入记录".to_owned()),
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
                // 输入法写入端缓冲里还有旧账号的输入：请它丢掉并重开
                request_input_log_reset();
            }
        }
        self.store(|path| AgentConfig::store_session(path, token, user_id, consents));
        tracing::info!("素笺云已登录");
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
                    tracing::warn!(%reason, "素笺云配置写不进去");
                    self.note = Some(reason);
                }
            }
            None => self.note = Some("找不到用户目录".to_owned()),
        }
        self.load_config();
    }
}

/// 拿到会话（建空间成功或配对批准）：取一次服务器上的开关（取不到就全关），包成事件。
fn signed_in_event(server: &str, grant: SessionGrant) -> AccountEvent {
    let consents = Client::new(server, &grant.token)
        .account()
        .map(|account| account.consents)
        .inspect_err(|error| tracing::warn!(%error, "拿到会话后取开关失败，先全关"))
        .unwrap_or_default();
    AccountEvent::SignedIn {
        token: grant.token,
        user_id: grant.user_id,
        consents,
    }
}

/// 输码之后等旧设备（手机）允许：每 2 秒问一次，到申请过期为止。`stop` 置位就立刻收手。
/// 单次网络失败不打断等待（网络抖一下不该让用户重输码），放弃是走到过期那一步的事。
fn poll_until_decided(server: &str, grant: &PairJoinGrant, stop: &AtomicBool) -> AccountEvent {
    let client = Client::anonymous(server);
    loop {
        if stop.load(Ordering::Acquire) {
            return AccountEvent::JoinFailed("已取消".to_owned());
        }
        if now_ms() >= grant.expires_at {
            return AccountEvent::JoinFailed("匹配码过期了，回手机上重新出一张".to_owned());
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
        if stop.load(Ordering::Acquire) {
            return AccountEvent::JoinFailed("已取消".to_owned());
        }
        match client.pair_poll(&grant.request_id, &grant.secret) {
            Ok(PairPoll::Approved(session)) => return signed_in_event(server, session),
            Ok(PairPoll::Denied) => {
                return AccountEvent::JoinFailed("手机上拒绝了这次加入".to_owned());
            }
            Ok(PairPoll::Pending) => {}
            Err(error) => tracing::debug!(%error, "配对轮询一次失败，继续等"),
        }
    }
}

/// 当前 Unix 毫秒。
fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}

fn spawn(name: &str, work: impl FnOnce() + Send + 'static) {
    if let Err(error) = std::thread::Builder::new()
        .name(name.to_owned())
        .spawn(work)
    {
        tracing::warn!(%error, thread = name, "后台线程起不来");
    }
}
