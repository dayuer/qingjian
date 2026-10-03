//! 轮询剪贴板变化。macOS 没有剪贴板变化通知，只能看变更计数；0.5 秒一次的开销可以忽略。

use crate::pasteboard;

pub struct ClipboardWatcher {
    /// 上次看到的变更计数。
    last: isize,
}

impl ClipboardWatcher {
    /// 从当前计数开始看，启动前已在剪贴板里的内容不上传。
    pub fn new() -> Self {
        Self {
            last: pasteboard::change_count(),
        }
    }

    /// 剪贴板变了且是可同步的文本就返回它。
    pub fn poll(&mut self) -> Option<String> {
        let count = pasteboard::change_count();
        if count == self.last {
            return None;
        }
        self.last = count;
        pasteboard::read_text().filter(|text| !text.trim().is_empty())
    }

    /// 本程序自己写入后调用：把这次变化当作已看过，防止把收到的内容又上传回去。
    pub fn note_own_write(&mut self, count: isize) {
        self.last = count;
    }
}
