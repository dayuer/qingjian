//! 菜单里显示的最近剪贴板，按收到的事件维护：新增插到最前，删除事件把对应条目拿掉。

mod entry;

use std::collections::VecDeque;

use qingjian_cloud_client::Incoming;
use qingjian_cloud_proto::EventKind;

pub use entry::Entry;

/// 菜单里最多列多少条。
const MAX_ENTRIES: usize = 15;

#[derive(Default)]
pub struct History {
    entries: VecDeque<Entry>,
}

impl History {
    /// 记下一条事件；返回列表有没有变化。
    pub fn apply(&mut self, incoming: &Incoming) -> bool {
        let event = &incoming.event;
        match &event.kind {
            EventKind::ClipAdded { text, .. } => {
                self.entries.push_front(Entry {
                    seq: event.seq,
                    device: event.device.clone(),
                    mine: incoming.mine,
                    text: text.clone(),
                });
                self.entries.truncate(MAX_ENTRIES);
                true
            }
            EventKind::ClipDeleted { target } => {
                let before = self.entries.len();
                self.entries.retain(|entry| entry.seq != *target);
                self.entries.len() != before
            }
            // 匹配码申请、设备加入是临时通知，不进剪贴板历史；提示由开通流程另做（客户端计划 Task 5）。
            EventKind::PairRequest { .. } | EventKind::DeviceJoined { .. } | EventKind::Unknown => {
                false
            }
        }
    }

    pub fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.entries.iter()
    }

    pub fn get(&self, index: usize) -> Option<&Entry> {
        self.entries.get(index)
    }
}
