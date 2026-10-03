//! 配置文件同步：输入法的 `config.toml`（设置与自定义短语）。整份按「后改的赢」，两边都改过时输的一方存成备份，不丢。
//! 输入法本来就会热加载 `config.toml`，写进去就生效，不需要输入法配合。

mod config_state;

use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use qingjian_cloud_proto::{ConfigDoc, PutConfig};

use crate::{Client, ClientError};

pub use config_state::ConfigState;

/// 输入法配置文件名。
const CONFIG_FILE: &str = "config.toml";

/// 冲突时输掉的那份放在输入法数据目录的这个子目录里。
const CONFLICT_DIR: &str = "sync/config-conflicts";

const STATE_FILE: &str = "config-state.json";

/// 一轮做了什么。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigOutcome {
    Unchanged,
    Uploaded,
    Downloaded,
    /// 两边都改过，保留了较新的一份，另一份存成备份。
    Conflict,
}

pub struct ConfigSync {
    client: Client,

    ime_dir: PathBuf,

    state_path: PathBuf,

    state: ConfigState,
}

impl ConfigSync {
    pub fn open(client: Client, ime_dir: &Path, state_dir: &Path) -> Result<Self, ClientError> {
        std::fs::create_dir_all(state_dir)?;
        let state_path = state_dir.join(STATE_FILE);
        let state = std::fs::read(&state_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        Ok(Self {
            client,
            ime_dir: ime_dir.to_owned(),
            state_path,
            state,
        })
    }

    fn config_path(&self) -> PathBuf {
        self.ime_dir.join(CONFIG_FILE)
    }

    pub fn cycle(&mut self) -> Result<ConfigOutcome, ClientError> {
        let local = match std::fs::read_to_string(self.config_path()) {
            Ok(text) => Some(text),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        let remote = self.client.config()?;
        let local_hash = local.as_deref().map(fnv1a);
        let local_changed = local_hash.is_some() && local_hash != self.state.hash;
        let remote_version = remote.as_ref().map_or(0, |doc| doc.version);
        let remote_changed = remote_version != self.state.version;

        match (local, remote) {
            (Some(text), _) if local_changed && !remote_changed => {
                self.upload(text, remote_version)?;
                Ok(ConfigOutcome::Uploaded)
            }
            (_, Some(doc)) if remote_changed && !local_changed => {
                self.download(&doc)?;
                Ok(ConfigOutcome::Downloaded)
            }
            (Some(text), Some(doc)) if local_changed && remote_changed => {
                if fnv1a(&doc.text) == fnv1a(&text) {
                    self.remember(doc.version, &text)?;
                    return Ok(ConfigOutcome::Unchanged);
                }
                let local_ms = modified_ms(&self.config_path());
                if local_ms > doc.at {
                    self.backup(&doc.text, &format!("server-{}", doc.device))?;
                    self.upload(text, doc.version)?;
                } else {
                    self.backup(&text, "local")?;
                    self.download(&doc)?;
                }
                Ok(ConfigOutcome::Conflict)
            }
            _ => Ok(ConfigOutcome::Unchanged),
        }
    }

    fn upload(&mut self, text: String, if_version: u64) -> Result<(), ClientError> {
        let doc = self.client.put_config(&PutConfig {
            text: text.clone(),
            if_version,
        })?;
        self.remember(doc.version, &text)
    }

    fn download(&mut self, doc: &ConfigDoc) -> Result<(), ClientError> {
        write_atomic(&self.config_path(), &doc.text)?;
        self.remember(doc.version, &doc.text)
    }

    fn backup(&self, text: &str, label: &str) -> Result<(), ClientError> {
        let stamp = std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or_default();
        let name: String = label
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let path = self
            .ime_dir
            .join(CONFLICT_DIR)
            .join(format!("config-{stamp}-{name}.toml"));
        tracing::warn!(path = %path.display(), "配置两边都改过，较旧的一份存成备份");
        write_atomic(&path, text)
    }

    fn remember(&mut self, version: u64, text: &str) -> Result<(), ClientError> {
        self.state = ConfigState {
            version,
            hash: Some(fnv1a(text)),
        };
        let bytes =
            serde_json::to_vec(&self.state).map_err(|e| ClientError::BadResponse(e.to_string()))?;
        std::fs::write(&self.state_path, bytes)?;
        Ok(())
    }
}

/// 文件内容指纹；只用来判断「变没变」，不需要密码学强度。
fn fnv1a(text: &str) -> u64 {
    text.bytes().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

fn modified_ms(path: &Path) -> i64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_millis() as i64)
}

fn write_atomic(path: &Path, text: &str) -> Result<(), ClientError> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    let temp = path.with_file_name(name);
    std::fs::write(&temp, text)?;
    std::fs::rename(&temp, path)?;
    Ok(())
}
