//! `QingjianCloud/config.toml`：服务器地址、登录得到的会话令牌与四个功能开关。第一次运行写出一份带注释的模板。
//! 令牌由菜单里的「登录…」写入、「退出登录」清空；开关在菜单里切换（先告诉服务器）。都经 `toml_edit` 改，注释保留。
//! 不并进输入法的 `config.toml`：那份会同步到别的设备，令牌是每台设备自己的。

use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use qingjian_cloud_proto::{Consents, TOKEN_PREFIX};
use serde::Deserialize;
use toml_edit::{DocumentMut, value};

/// 模板里的服务器，也是配置里没写地址时用的。
pub const DEFAULT_SERVER: &str = "https://pinyin.synon.ai";

/// 第一次运行写出的模板。令牌只在本机，文件权限设成仅本人可读。
const TEMPLATE: &str = r#"# 素笺云配置。开通、加入与功能开关都在输入法「中☁ → 素笺云 ›」里操作，一般不用手改。
# 服务器地址，带 https://
server = "https://pinyin.synon.ai"

# 会话令牌：由菜单里的开通 / 加入写入，「解绑这台 Mac」清空。不要手填，也不要发给别人
token = ""

# 账号 id，开通或加入时写入；换账号时据此清掉旧账号的同步进度。不要手改
# user_id = 0

# 以下四项与服务器上的同意记录一致，在菜单里切换（会先告诉服务器）；手改不会同步到服务器
# 跨设备剪贴板
clipboard = false

# 同步输入法的学习数据与 config.toml（设置与自定义短语）
sync = false

# 上传输入法的输入日志（input-log.jsonl）
logs = false

# 云联想走服务器的大模型
llm = false

# 开了 logs 时，把别的设备的输入日志下载到 ~/Library/Application Support/QingjianCloud/input-log/
download_logs = true
"#;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct AgentConfig {
    pub server: String,

    /// 会话令牌（`sjt_` 开头），由登录写入。旧版手填的设备令牌（`qjc_`）已作废，当没登录。
    pub token: String,

    /// 跨设备剪贴板（服务器上的 `clipboard`）。
    pub clipboard: bool,

    /// 同步学习数据与 `config.toml`（服务器上的 `sync`）。
    pub sync: bool,

    /// 上传输入日志（服务器上的 `input_log`）。
    pub logs: bool,

    /// 云联想走服务器的大模型（服务器上的 `llm`）。
    pub llm: bool,

    /// 素材上传（服务器上的 `memory`）。Mac 暂不用，只记着，免得与服务器的开关反复对不上。
    pub memory: bool,

    /// 开了 `logs` 时下载别的设备的输入日志；只在本机，不经服务器。
    pub download_logs: bool,

    /// 登录的账号 id。退出登录时留着，换账号后才变，用来判断同步进度要不要作废；旧文件里没有。
    pub user_id: Option<i64>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            server: String::new(),
            token: String::new(),
            clipboard: false,
            sync: false,
            logs: false,
            llm: false,
            memory: false,
            download_logs: true,
            user_id: None,
        }
    }
}

impl AgentConfig {
    /// 读配置；文件不存在就写模板并按模板读。读不了或格式不对返回 `Err`，带给用户看的原因。没登录不算错。
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if let Err(reason) = write_private(path, TEMPLATE) {
                    tracing::warn!(%reason, "配置模板写入失败");
                }
                TEMPLATE.to_owned()
            }
            Err(error) => return Err(format!("配置文件读不了：{error}")),
        };
        toml::from_str(&text).map_err(|error| parse_failure(&text, error.span(), error.message()))
    }

    pub fn signed_in(&self) -> bool {
        self.token.trim().starts_with(TOKEN_PREFIX)
    }

    /// 去掉末尾的 `/`；没写地址时用 [`DEFAULT_SERVER`]。
    pub fn server(&self) -> String {
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
            memory: self.memory,
        }
    }

    /// 登录成功：写入令牌、账号 id 与服务器上的开关。
    pub fn store_session(
        path: &Path,
        token: &str,
        user_id: i64,
        consents: Consents,
    ) -> Result<(), String> {
        edit(path, |document| {
            document["token"] = value(token);
            document["user_id"] = value(user_id);
            set_consents(document, consents);
        })
    }

    /// 服务器上的开关变了：只改开关。
    pub fn store_consents(path: &Path, consents: Consents) -> Result<(), String> {
        edit(path, |document| set_consents(document, consents))
    }

    /// 退出登录、令牌失效：清掉令牌与开关，`user_id` 留着（同一账号重登保留进度）。
    pub fn clear_session(path: &Path) -> Result<(), String> {
        edit(path, |document| {
            document["token"] = value("");
            set_consents(document, Consents::default());
        })
    }
}

fn set_consents(document: &mut DocumentMut, consents: Consents) {
    document["clipboard"] = value(consents.clipboard);
    document["sync"] = value(consents.sync);
    document["logs"] = value(consents.input_log);
    document["llm"] = value(consents.llm);
    document["memory"] = value(consents.memory);
}

/// 读出文件（没有就从模板开始）、改、写回，注释与别的键原样保留。
fn edit(path: &Path, change: impl FnOnce(&mut DocumentMut)) -> Result<(), String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => TEMPLATE.to_owned(),
        Err(error) => return Err(format!("配置文件读不了：{error}")),
    };
    let mut document: DocumentMut = text.parse().map_err(|error: toml_edit::TomlError| {
        parse_failure(&text, error.span(), error.message())
    })?;
    change(&mut document);
    write_private(path, &document.to_string())
}

/// 配置解析失败给用户与日志看的话：只有行号与原因，绝不带源码片段。
/// `toml` 错误的 `Display` 会带出错行原文，那一行可能就是 `token = "sjt_…"`；`message` 里偶尔含值（类型不符时），
/// 所以在第一个引号或反引号处截断。
fn parse_failure(text: &str, span: Option<std::ops::Range<usize>>, message: &str) -> String {
    let message = match message.find(['"', '`', '\'']) {
        Some(at) => format!("{}…", message[..at].trim_end()),
        None => message.to_owned(),
    };
    match span {
        Some(span) => {
            let upto = span.start.min(text.len());
            let line = text.as_bytes()[..upto]
                .iter()
                .filter(|&&b| b == b'\n')
                .count()
                + 1;
            format!("配置文件第 {line} 行格式不对：{message}")
        }
        None => format!("配置文件格式不对：{message}"),
    }
}

/// 临时文件名里的递增计数。
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// 原子地写文件并设成仅本人可读写（里面有令牌）：同目录建 0600 的临时文件、写满、落盘后改名覆盖，
/// 读的一方（输入法重载配置）不会看到写了一半的文件，旧文件的宽权限也不会带过来。
fn write_private(path: &Path, text: &str) -> Result<(), String> {
    write_private_io(path, text).map_err(|error| error.to_string())
}

fn write_private_io(path: &Path, text: &str) -> std::io::Result<()> {
    let dir = path.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir)?;
    let serial = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp = dir.join(format!(
        ".{}.{}.{serial}.tmp",
        path.file_name().unwrap_or_default().to_string_lossy(),
        std::process::id()
    ));
    // 崩溃遗留的同名临时文件可能是宽权限的，先删掉再 create_new
    match std::fs::remove_file(&tmp) {
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
    let written = (|| {
        let mut file = options.open(&tmp)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        }
        file.write_all(text.as_bytes())?;
        file.sync_all()?;
        std::fs::rename(&tmp, path)
    })();
    if written.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    written
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("qjc-mac-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("config.toml")
    }

    #[test]
    fn missing_file_writes_template_that_reads_as_signed_out() {
        let path = temp("template");
        let config = AgentConfig::load(&path).unwrap();
        assert!(!config.signed_in());
        assert_eq!(config.server(), DEFAULT_SERVER);
        assert_eq!(config.consents(), Consents::default());
        assert_eq!(config.user_id, None);
        assert!(config.download_logs);
        assert!(
            std::fs::read_to_string(&path)
                .unwrap()
                .contains("token = \"\"")
        );
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn old_device_token_is_signed_out() {
        let path = temp("old");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "server = \"https://x/\"\ntoken = \"qjc_1\"\nlearning = true\n",
        )
        .unwrap();
        let config = AgentConfig::load(&path).unwrap();
        assert!(!config.signed_in());
        assert_eq!(config.server(), "https://x");
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn store_session_keeps_comments_and_sets_switches() {
        let path = temp("session");
        AgentConfig::load(&path).unwrap();
        let consents = Consents {
            sync: true,
            llm: true,
            ..Consents::default()
        };
        AgentConfig::store_session(&path, "sjt_x", 7, consents).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("# 素笺云配置"));
        let config = AgentConfig::load(&path).unwrap();
        assert!(config.signed_in());
        assert_eq!(config.token, "sjt_x");
        assert_eq!(config.user_id, Some(7));
        assert_eq!(config.consents(), consents);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn store_consents_and_clear_session_keep_user_id() {
        let path = temp("clear");
        AgentConfig::store_session(&path, "sjt_x", 7, Consents::default()).unwrap();
        let consents = Consents {
            clipboard: true,
            input_log: true,
            ..Consents::default()
        };
        AgentConfig::store_consents(&path, consents).unwrap();
        let config = AgentConfig::load(&path).unwrap();
        assert_eq!(config.token, "sjt_x");
        assert!(config.clipboard && config.logs && !config.sync);
        AgentConfig::clear_session(&path).unwrap();
        let config = AgentConfig::load(&path).unwrap();
        assert!(!config.signed_in());
        assert_eq!(config.consents(), Consents::default());
        // 同一账号重登要认得出来
        assert_eq!(config.user_id, Some(7));
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn user_id_changes_with_the_next_login() {
        let path = temp("switch");
        AgentConfig::store_session(&path, "sjt_a", 7, Consents::default()).unwrap();
        AgentConfig::clear_session(&path).unwrap();
        AgentConfig::store_session(&path, "sjt_b", 8, Consents::default()).unwrap();
        assert_eq!(AgentConfig::load(&path).unwrap().user_id, Some(8));
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn parse_failure_never_contains_source_text() {
        let path = temp("badtoml");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        for text in [
            "server = \"x\"\ntoken = \"sjt_SECRET\" oops\n",
            "token = \"sjt_SECRET\"\nclipboard = \"sjt_SECRET\"\n",
            "token = \"sjt_SECRET\n",
        ] {
            std::fs::write(&path, text).unwrap();
            let load = AgentConfig::load(&path).unwrap_err();
            assert!(load.starts_with("配置文件第 "), "{load}");
            assert!(!load.contains("sjt_"), "{load}");
            // 只是类型不对的文件 toml_edit 能改；语法错的改不了，报错同样不带原文
            if let Err(edit) = AgentConfig::store_consents(&path, Consents::default()) {
                assert!(!edit.contains("sjt_"), "{edit}");
            }
        }
        std::fs::write(&path, "server = \"x\"\ntoken = \"sjt_SECRET\" oops\n").unwrap();
        assert!(
            AgentConfig::load(&path)
                .unwrap_err()
                .starts_with("配置文件第 2 行格式不对：")
        );
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn write_private_replaces_atomically_with_0600() {
        let path = temp("atomic");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            "a very long old content that is longer than the new one\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        }
        write_private(&path, "short\n").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "short\n");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        let leftovers: Vec<_> = std::fs::read_dir(path.parent().unwrap())
            .unwrap()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(leftovers, ["config.toml"]);
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
