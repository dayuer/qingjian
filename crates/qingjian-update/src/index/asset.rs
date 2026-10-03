use serde::Deserialize;

use crate::Package;

/// 一个安装包：只看它是给哪个平台、哪种 CPU 的。
#[derive(Debug, Clone, Deserialize)]
pub struct Asset {
    pub platform: String,

    /// `arm64` / `x86_64`；0.1.3 之前的索引没有这个字段。
    #[serde(default)]
    pub cpu: String,

    /// 文件名、下载地址与校验和：自动下载用，缺任何一个就只提示不下载。
    #[serde(default)]
    pub file: String,

    #[serde(default)]
    pub url: String,

    #[serde(default)]
    pub sha256: String,
}

impl Asset {
    /// 文件名、地址、校验和都齐了才算能下载；文件名只取最后一段，防索引里带路径。
    pub(crate) fn package(&self) -> Option<Package> {
        let file = self.file.rsplit('/').next().unwrap_or_default();
        if file.is_empty() || self.url.is_empty() || self.sha256.len() != 64 {
            return None;
        }
        Some(Package {
            file: file.to_owned(),
            url: self.url.clone(),
            sha256: self.sha256.to_ascii_lowercase(),
        })
    }
}
