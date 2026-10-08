//! 键盘工具栏「记录中」标记的四种状态，C 接口按 u8 传（见 qingjian_bridge.h）。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RecordingState {
    /// 不显示：没登录、没开「上传输入日志」。
    Off = 0,
    Recording = 1,
    PausedTimed = 2,
    PausedForever = 3,
}
