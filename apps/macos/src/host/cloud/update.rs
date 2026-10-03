//! 自建更新服务器：新版本的安装包已在后台下载并校验过时，「有新版本」直接打开它（系统安装程序），
//! 否则照旧打开下载页。不自动弹出安装程序，免得打断打字。自用分叉补丁，见 `cloud/docs/fork-patch.md`。

use crate::host::Host;
use crate::host::diagnostics::open_with_system;

impl Host {
    pub(in crate::host) fn open_update(&self) {
        let package = self
            .updates
            .as_ref()
            .and_then(|updates| updates.downloaded(&self.settings.config().update));
        match package {
            Some(path) => {
                tracing::info!(path = %path.display(), "打开新版本安装包");
                open_with_system(&[&path.to_string_lossy()]);
            }
            None => open_with_system(&[qingjian_update::DOWNLOAD_URL]),
        }
    }
}
