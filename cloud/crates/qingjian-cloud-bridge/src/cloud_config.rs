//! 键盘连青简 Cloud 的配置（`cloud.toml`，在 App Group 里）：服务器地址、登录得到的会话令牌与四个功能开关。
//! 地址构建时写进随包的种子；令牌只由登录写入（见 `account`），退出登录、删账号时清空；
//! 开关跟着服务器上的同意记录走，缺省全关。没有这个文件、地址为空或没登录，键盘就完全离线。

use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use qingjian_cloud_proto::{Consents, TOKEN_PREFIX};
use serde::{Deserialize, Serialize};

/// 进程内串起 `cloud.toml` 的读-改-写：Swift 可能在不同后台线程同时调账号接口。网络请求不要在锁里做。
static WRITE_LOCK: Mutex<()> = Mutex::new(());

/// 临时文件名里的递增计数。
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// `cloud.toml` 里没写地址时用的服务器。
pub const DEFAULT_SERVER: &str = "https://pinyin.synon.ai";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CloudConfig {
    /// 服务器地址，构建时写进随包的 `cloud.toml`。
    pub server: String,

    /// 登录得到的会话令牌（`sjt_` 开头）。旧版的设备令牌（`qjc_`）已作废，当没登录。
    pub token: String,

    /// 登录的账号 id。退出登录时留着，换账号或删号后才变，用来判断同步进度要不要作废；旧文件里没有。
    pub user_id: Option<i64>,

    /// 用服务器的大模型：润色，以及 `config.toml` 里 `[predict]` 开着时的云联想（服务器上的 `llm`）。
    pub llm: bool,

    /// 记输入日志并上传（服务器上的 `input_log`）。
    pub logs: bool,

    /// 与别的设备同步学习数据与 `config.toml`（服务器上的 `sync`）。
    pub sync: bool,

    /// 跨设备剪贴板（服务器上的 `clipboard`）。
    pub clipboard: bool,
}

impl CloudConfig {
    /// 键盘用：读不了、格式不对、没地址或没登录都当没配置，键盘照常离线用。
    pub fn load(path: &Path) -> Option<Self> {
        let config = Self::read(path)?;
        (!config.server.trim().is_empty() && config.signed_in()).then_some(config)
    }

    /// 原样读出（没登录也读），读不了返回 `None`。
    pub fn read(path: &Path) -> Option<Self> {
        let text = std::fs::read_to_string(path).ok()?;
        toml::from_str(&text)
            .inspect_err(|error| {
                tracing::warn!(reason = %parse_failure_note(error), "cloud.toml 格式不对，按离线用");
            })
            .ok()
    }

    pub fn signed_in(&self) -> bool {
        self.token.trim().starts_with(TOKEN_PREFIX)
    }

    /// 去掉末尾的 `/`；没写地址时用 [`DEFAULT_SERVER`]。
    pub fn server_or_default(&self) -> String {
        let server = self.server.trim().trim_end_matches('/');
        if server.is_empty() {
            DEFAULT_SERVER.to_owned()
        } else {
            server.to_owned()
        }
    }

    pub fn consents(&self) -> Consents {
        Consents {
            clipboard: self.clipboard,
            sync: self.sync,
            input_log: self.logs,
            llm: self.llm,
        }
    }

    pub fn set_consents(&mut self, consents: Consents) {
        self.clipboard = consents.clipboard;
        self.sync = consents.sync;
        self.logs = consents.input_log;
        self.llm = consents.llm;
    }

    /// 登录成功：写入地址、令牌与服务器上的开关。
    pub fn store_session(
        path: &Path,
        server: &str,
        token: &str,
        user_id: i64,
        consents: Consents,
    ) -> Result<(), String> {
        let _guard = lock();
        let mut config = Self::read(path).unwrap_or_default();
        config.server = server.to_owned();
        config.token = token.to_owned();
        config.user_id = Some(user_id);
        config.set_consents(consents);
        config.save(path)
    }

    /// 服务器上的开关变了：只改开关，地址与令牌不动。读不到现有文件就报错，不用默认值写出一份没有令牌的把登录状态清掉。
    pub fn store_consents(path: &Path, consents: Consents) -> Result<(), String> {
        let _guard = lock();
        let mut config = Self::read(path).ok_or_else(|| "cloud.toml 读不了".to_owned())?;
        config.set_consents(consents);
        config.save(path)
    }

    /// 退出登录、令牌失效：清掉令牌与开关，地址与 `user_id` 留着。没有文件也算成功。
    pub fn clear_session(path: &Path) -> Result<(), String> {
        let _guard = lock();
        let Some(mut config) = Self::read(path) else {
            return Ok(());
        };
        config.token.clear();
        config.set_consents(Consents::default());
        config.save(path)
    }

    /// 删账号成功：在 [`CloudConfig::clear_session`] 之外连 `user_id` 也忘掉，之后再登录一律按换账号处理。
    pub fn clear_account(path: &Path) -> Result<(), String> {
        let _guard = lock();
        let Some(mut config) = Self::read(path) else {
            return Ok(());
        };
        config.token.clear();
        config.user_id = None;
        config.set_consents(Consents::default());
        config.save(path)
    }

    /// 整份写回（这份文件只有这几项，不用保留注释）。里面有令牌：同目录写 `.tmp`（名字带进程号与序号，Unix 上 0600）再改名，
    /// 键盘随时在读，不能让它读到写了一半的文件。
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let text = toml::to_string(self).map_err(|e| e.to_string())?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let mut name = path.file_name().unwrap_or_default().to_os_string();
        let serial = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
        name.push(format!(".{}.{serial}.tmp", std::process::id()));
        let temp = path.with_file_name(name);
        let written =
            write_private(&temp, text.as_bytes()).and_then(|()| std::fs::rename(&temp, path));
        written.map_err(|e| {
            std::fs::remove_file(&temp).ok();
            e.to_string()
        })
    }

    /// 大模型代理的接口地址（OpenAI 兼容，不含 `/chat/completions`）。
    pub fn llm_base_url(&self) -> String {
        format!("{}/v1", self.server_or_default())
    }
}

fn lock() -> std::sync::MutexGuard<'static, ()> {
    WRITE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// 解析失败的日志文案：只有原因与出错位置。`toml` 错误的 `Display` 会带出错行原文，那一行可能就是令牌。
pub fn parse_failure_note(error: &toml::de::Error) -> String {
    match error.span() {
        Some(span) => format!("{}（第 {} 字节）", error.message(), span.start),
        None => error.message().to_owned(),
    }
}

fn write_private(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;

    // 崩溃遗留的 tmp 可能是宽权限的，先删掉再建，免得带令牌的文件继承它
    match std::fs::remove_file(path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    }
    file.write_all(bytes)?;
    file.sync_all()
}
