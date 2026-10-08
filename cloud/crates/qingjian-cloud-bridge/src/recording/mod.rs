//! 「记录中」标记的暂停状态：暂停文件与四种状态。

mod pause;
mod state;

pub use pause::{RecordingPause, Until};
pub use state::RecordingState;
