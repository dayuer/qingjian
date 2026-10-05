//! 会话手里的换层把手。引擎拿走了 `Box<dyn Learner>`，`Engine::learner_mut()` 只给 `&mut dyn Learner`、没法向下转型，
//! 所以叠加层放在与 [`super::ScopedLearner`] 共享的 `Arc<Mutex<_>>` 里，由这里换。

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use super::{Overlay, lock};

#[derive(Debug, Clone)]
pub struct ScopeHandle {
    overlay: Arc<Mutex<Overlay>>,

    memory_dir: PathBuf,
}

impl ScopeHandle {
    pub(super) fn new(overlay: Arc<Mutex<Overlay>>, memory_dir: PathBuf) -> Self {
        Self {
            overlay,
            memory_dir,
        }
    }

    /// 换到 `contact`：旧叠加层先落盘再从磁盘开新的，换回同一对象时读得到刚记的。
    /// 换完调用方要调一次 `Engine::learner_mut()`，作废格子缓存里按旧叠加层排的候选。
    pub fn switch(&self, contact: Option<&str>) {
        let mut overlay = lock(&self.overlay);
        overlay.flush();
        *overlay = Overlay::open(&self.memory_dir, contact);
    }
}
