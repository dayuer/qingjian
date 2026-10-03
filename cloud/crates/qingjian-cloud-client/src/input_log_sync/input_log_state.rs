//! 输入日志同步的进度。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputLogState {
    /// 本机日志已上传到的字节偏移。
    pub offset: u64,

    /// 上传时文件开头的内容；变了说明文件被清空后又写了新内容。
    pub head: String,

    /// 服务器的清空代数；批号里带着它，清空后从偏移 0 重传不会撞上旧批号。
    pub generation: u64,

    /// 下载别的设备的日志拉到的 `seq`。
    pub cursor: u64,

    /// 本机的设备名，下载时跳过自己的。
    pub device: Option<String>,
}
