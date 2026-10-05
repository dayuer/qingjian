//! [`should_upload`](super::should_upload) 的结论。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UploadDecision {
    /// 把 `uploaded == false` 的素材交给 2B 上传。
    Upload,

    /// 这一轮不传，素材留在本机等条件满足。
    Hold,

    /// 停掉正在排着的上传（同意被撤回），素材留在本机。
    Stop,
}
