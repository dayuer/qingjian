//! `config.toml`：服务器地址与设备令牌。第一次运行写出一份带注释的模板，用户填好后菜单里点「重新加载配置」。

use std::path::Path;

use serde::Deserialize;

/// 第一次运行写出的模板。令牌只在本机，文件权限设成仅本人可读。
const TEMPLATE: &str = r#"# 青简 Cloud 配置。填好后在输入法「中☁」菜单的「青简 Cloud」子菜单里点「重新加载配置」。
# 服务器地址（deploy/.env 里的 QINGJIAN_DOMAIN），带 https://
server = ""

# 在服务器上运行 `docker compose exec cloud qingjian-cloud device add <设备名>` 得到的令牌
token = ""

# 同步输入法的学习数据（词频、选择、n-gram、敲错、英文词、用户词）；需要打了 Cloud 补丁的青简
learning = true

# 同步输入法的 config.toml（设置与自定义短语）；两台设备都改过时较新的赢，另一份存成备份
settings = true

# 上传输入法的输入日志（input-log.jsonl）到服务器
logs = true

# 把别的设备的输入日志下载到 ~/Library/Application Support/QingjianCloud/input-log/
download_logs = true

# 在菜单栏显示单独的图标；缺省 false，状态与操作在输入法「中☁」菜单的「青简 Cloud」子菜单里
menu_bar = false
"#;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct AgentConfig {
    pub server: String,

    pub token: String,

    /// 同步学习数据。
    pub learning: bool,

    /// 同步 `config.toml`。
    pub settings: bool,

    /// 上传输入日志。
    pub logs: bool,

    /// 下载别的设备的输入日志。
    pub download_logs: bool,

    /// 在菜单栏显示自己的图标。缺省关：状态与操作在输入法「中☁」菜单的「青简 Cloud」子菜单里。
    pub menu_bar: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            server: String::new(),
            token: String::new(),
            learning: true,
            settings: true,
            logs: true,
            download_logs: true,
            menu_bar: false,
        }
    }
}

impl AgentConfig {
    /// 读配置；文件不存在就写模板。读不了或没填完返回 `Err`，带给用户看的原因。
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                write_template(path);
                return Err("未配置：在配置文件里填服务器与令牌".to_owned());
            }
            Err(error) => return Err(format!("配置文件读不了：{error}")),
        };
        let config: Self =
            toml::from_str(&text).map_err(|error| format!("配置文件有错：{error}"))?;
        if config.server.trim().is_empty() || config.token.trim().is_empty() {
            return Err("未配置：在配置文件里填服务器与令牌".to_owned());
        }
        Ok(config)
    }
}

fn write_template(path: &Path) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Err(error) = std::fs::write(path, TEMPLATE) {
        tracing::warn!(%error, "配置模板写入失败");
        return;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_writes_template_that_reads_as_unconfigured() {
        let dir = std::env::temp_dir().join(format!("qjc-mac-{}", std::process::id()));
        let path = dir.join("config.toml");
        let _ = std::fs::remove_file(&path);
        assert!(AgentConfig::load(&path).unwrap_err().starts_with("未配置"));
        assert!(AgentConfig::load(&path).unwrap_err().starts_with("未配置"));
        std::fs::write(&path, "server = \"https://x\"\ntoken = \"qjc_1\"\n").unwrap();
        assert_eq!(AgentConfig::load(&path).unwrap().token, "qjc_1");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
