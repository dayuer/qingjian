//! 让输入法的云联想走 Cloud 的大模型代理：改输入法 `config.toml` 的 `[predict]`，令牌写进输入法的 `.env`。
//!
//! 令牌按设备不同，所以不写进会被同步到别的设备的 `config.toml`：配置里只写环境变量名，
//! 每台设备的常驻程序各自把自己的令牌写进本机 `.env`（不同步）。输入法热加载配置时会补读 `.env` 里新加的变量。

use std::path::Path;

use toml_edit::{DocumentMut, Item, Table, value};

/// 存设备令牌的环境变量名。
pub const TOKEN_ENV: &str = "QINGJIAN_CLOUD_TOKEN";

/// 把 `[predict]` 指向 `server` 的代理并打开云联想；其余设置（模型、格数、防抖）不动。
pub fn use_cloud_llm(config_path: &Path, server: &str) -> Result<(), String> {
    let text = match std::fs::read_to_string(config_path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(format!("读不了输入法配置：{error}")),
    };
    let mut doc: DocumentMut = text
        .parse()
        .map_err(|e| format!("输入法配置有语法错误：{e}"))?;
    if !doc.contains_table("predict") {
        doc["predict"] = Item::Table(Table::new());
    }
    let predict = &mut doc["predict"];
    predict["enabled"] = value(true);
    predict["base_url"] = value(format!("{}/v1", server.trim().trim_end_matches('/')));
    predict["api_key"] = value("");
    predict["api_key_env"] = value(TOKEN_ENV);
    write_atomic(config_path, &doc.to_string(), false)
}

/// 输入法的 `.env` 里写上本设备的令牌（已是同样的值就不动），其余行保留，权限 0600。
pub fn ensure_token(env_path: &Path, token: &str) -> Result<bool, String> {
    let existing = std::fs::read_to_string(env_path).unwrap_or_default();
    let entry = format!("{TOKEN_ENV}={}", token.trim());
    if existing.lines().any(|line| line.trim() == entry) {
        return Ok(false);
    }
    let prefix = format!("{TOKEN_ENV}=");
    let mut lines: Vec<&str> = existing
        .lines()
        .filter(|line| !line.trim_start().starts_with(&prefix))
        .collect();
    lines.push(&entry);
    write_atomic(env_path, &format!("{}\n", lines.join("\n")), true)?;
    Ok(true)
}

fn write_atomic(path: &Path, text: &str, private: bool) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    let temp = path.with_file_name(name);
    std::fs::write(&temp, text).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    if private {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&temp, std::fs::Permissions::from_mode(0o600))
            .map_err(|e| e.to_string())?;
    }
    #[cfg(not(unix))]
    let _ = private;
    std::fs::rename(&temp, path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_predict_and_keeps_everything_else() {
        let dir = tempfile_dir();
        let path = dir.join("config.toml");
        std::fs::write(
            &path,
            "# 我的配置\n[general]\nshuangpin = \"xiaohe\"\n\n[predict]\nenabled = false\nmodel = \"deepseek-v4-flash\"\napi_key = \"sk-old\"\nslots = 3\n",
        )
        .unwrap();
        use_cloud_llm(&path, "https://cloud.example.com/").unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("# 我的配置\n[general]\nshuangpin = \"xiaohe\"\n"));
        let doc: DocumentMut = text.parse().unwrap();
        assert_eq!(doc["predict"]["enabled"].as_bool(), Some(true));
        assert_eq!(
            doc["predict"]["base_url"].as_str(),
            Some("https://cloud.example.com/v1")
        );
        assert_eq!(doc["predict"]["api_key"].as_str(), Some(""));
        assert_eq!(doc["predict"]["api_key_env"].as_str(), Some(TOKEN_ENV));
        assert_eq!(doc["predict"]["model"].as_str(), Some("deepseek-v4-flash"));
        assert_eq!(doc["predict"]["slots"].as_integer(), Some(3));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn token_replaces_only_its_own_line() {
        let dir = tempfile_dir();
        let path = dir.join(".env");
        std::fs::write(&path, "QINGJIAN_API_KEY=sk-x\nQINGJIAN_CLOUD_TOKEN=old\n").unwrap();
        assert!(ensure_token(&path, "qjc_new").unwrap());
        assert!(!ensure_token(&path, "qjc_new").unwrap());
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "QINGJIAN_API_KEY=sk-x\nQINGJIAN_CLOUD_TOKEN=qjc_new\n"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    fn tempfile_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "qjc-ime-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
