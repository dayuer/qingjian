//! 查到新版本后把安装包下载到数据目录的 `updates/`，sha256 对上才留下；壳只负责在用户点了之后打开它。
//! 自用分叉补丁，见 `cloud/docs/fork-patch.md`。

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use sha2::{Digest, Sha256};

use crate::{Package, UpdateError};

/// 安装包上限，防索引写错时把磁盘写满。
const MAX_PACKAGE_BYTES: u64 = 1024 * 1024 * 1024;

const TIMEOUT: Duration = Duration::from_secs(30 * 60);

/// 已下载且校验过就直接返回；否则下载到临时文件，校验通过再改名。`updates/` 里别的旧包顺手删掉。
pub(crate) fn fetch_package(
    dir: &Path,
    package: &Package,
    current_version: &str,
) -> Result<PathBuf, UpdateError> {
    std::fs::create_dir_all(dir)?;
    let target = dir.join(&package.file);
    remove_others(dir, &package.file);
    if target.is_file() && sha256_file(&target)? == package.sha256 {
        return Ok(target);
    }
    let partial = dir.join(format!("{}.part", package.file));
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let digest = runtime.block_on(download(&package.url, &partial, current_version))?;
    if digest != package.sha256 {
        let _ = std::fs::remove_file(&partial);
        return Err(UpdateError::ChecksumMismatch);
    }
    std::fs::rename(&partial, &target)?;
    Ok(target)
}

async fn download(url: &str, path: &Path, current_version: &str) -> Result<String, UpdateError> {
    let client = reqwest::Client::builder()
        .timeout(TIMEOUT)
        .user_agent(format!("qingjian/{current_version}"))
        .build()?;
    let mut response = client.get(url).send().await?.error_for_status()?;
    let mut file = std::fs::File::create(path)?;
    let mut hasher = Sha256::new();
    let mut written = 0u64;
    while let Some(chunk) = response.chunk().await? {
        written += chunk.len() as u64;
        if written > MAX_PACKAGE_BYTES {
            drop(file);
            let _ = std::fs::remove_file(path);
            return Err(UpdateError::TooLarge(MAX_PACKAGE_BYTES as usize));
        }
        hasher.update(&chunk);
        file.write_all(&chunk)?;
    }
    file.sync_all()?;
    Ok(hex(&hasher.finalize()))
}

fn sha256_file(path: &Path) -> Result<String, UpdateError> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = vec![0u8; 1 << 16];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex(&hasher.finalize()))
}

fn remove_others(dir: &Path, keep: &str) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if entry.file_name().to_str() != Some(keep) {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_a_verified_package_and_removes_stale_ones() {
        let dir = std::env::temp_dir().join(format!("qingjian-update-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("old.pkg"), b"old").unwrap();
        std::fs::write(dir.join("new.pkg"), b"new").unwrap();
        let package = Package {
            file: "new.pkg".to_owned(),
            url: "http://127.0.0.1:9/unreachable".to_owned(),
            sha256: sha256_file(&dir.join("new.pkg")).unwrap(),
        };
        // 已有且校验对得上：不联网直接用
        assert_eq!(
            fetch_package(&dir, &package, "0.0.0").unwrap(),
            dir.join("new.pkg")
        );
        assert!(!dir.join("old.pkg").exists());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
