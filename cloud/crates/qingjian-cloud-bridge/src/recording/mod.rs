//! 「记录中」标记的暂停状态：暂停文件与四种状态。

// TODO(Task 2)：会话与 C 接口接上这两个 re-export 之后删掉。
#![allow(unused_imports)]

mod pause;
mod state;

pub use pause::{RecordingPause, Until};
pub use state::RecordingState;
