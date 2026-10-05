//! 当前生效的叠加层：只在选了对象时开对象层；选了人时写只进对象层（见 [`super::ScopedLearner`]）。
//! 场景自 2026-10-05 起只是用户自建的分组，没有场景层，也没有恋爱那种「写只进叠加层」的排他层。

use std::path::Path;

use qingjian_core::Learner;
use qingjian_learning::FrequencyLearner;

use super::{contact_learning_dir, is_contact_id, load_layer};

#[derive(Debug, Default)]
pub struct Overlay {
    contact: Option<FrequencyLearner>,

    /// 换过几次层：[`super::ScopedLearner`] 拿它判断手里的用户词快照是不是这一层的。
    generation: u64,
}

impl Overlay {
    /// 给了合格的对象 id、且对象目录还在时才开对象层。
    /// 对象目录只由建对象时创建，这里不建：忘掉的人不会因为键盘还选着它而被重新建出来。
    pub fn open(memory_dir: &Path, contact: Option<&str>) -> Self {
        Self {
            contact: contact
                .filter(|id| is_contact_id(id) && memory_dir.join(id).is_dir())
                .map(|id| load_layer(&contact_learning_dir(memory_dir, id))),
            generation: 0,
        }
    }

    /// 换成 `contact` 的层：旧层先落盘再开新的，代数加一。
    pub fn replace(&mut self, memory_dir: &Path, contact: Option<&str>) {
        self.flush();
        let generation = self.generation.wrapping_add(1);
        *self = Self::open(memory_dir, contact);
        self.generation = generation;
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// 对象层（没选人时为 `None`）。
    pub fn contact(&self) -> Option<&FrequencyLearner> {
        self.contact.as_ref()
    }

    /// 选了人（且对象目录在）时是这个人的「隔离层」：写只进它、不碰全局，也不记个人 n-gram。
    /// 对象目录不在（被忘掉了）时当没选人。
    pub fn isolated(&self) -> bool {
        self.contact.is_some()
    }

    /// 各叠加层的计数乘 `weight` 再相加。
    pub fn count(&self, weight: u32, read: impl Fn(&FrequencyLearner) -> u32) -> u32 {
        self.contact
            .as_ref()
            .map_or(0, |layer| weight.saturating_mul(read(layer)))
    }

    pub fn write(&mut self, mut f: impl FnMut(&mut FrequencyLearner)) {
        if let Some(layer) = self.contact.as_mut() {
            f(layer);
        }
    }

    pub fn flush(&mut self) {
        self.write(|layer| layer.flush());
    }
}
