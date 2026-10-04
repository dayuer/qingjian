//! 当前生效的叠加层：只在恋爱场景有；场景层必有，对象层看选没选对象。

use std::path::Path;

use qingjian_cloud_proto::Scene;
use qingjian_core::Learner;
use qingjian_learning::FrequencyLearner;

use super::{contact_learning_dir, is_contact_id, load_layer, scene_learning_dir};

#[derive(Debug, Default)]
pub struct Overlay {
    scene: Option<FrequencyLearner>,

    contact: Option<FrequencyLearner>,
}

impl Overlay {
    /// 日常与工作没有叠加层；恋爱场景开场景层，给了合格的对象 id、且对象目录还在时再开对象层。
    /// 对象目录只由建对象时创建，这里不建：忘掉的人不会因为键盘还选着它而被重新建出来。
    pub fn open(memory_dir: &Path, scene: Scene, contact: Option<&str>) -> Self {
        if scene != Scene::Dating {
            return Self::default();
        }
        let contact = contact
            .filter(|id| is_contact_id(id) && memory_dir.join(id).is_dir())
            .map(|id| load_layer(&contact_learning_dir(memory_dir, id)));
        Self {
            scene: Some(load_layer(&scene_learning_dir(memory_dir, scene))),
            contact,
        }
    }

    /// 有叠加层（恋爱场景）时，写只进叠加层。
    pub fn active(&self) -> bool {
        self.scene.is_some()
    }

    /// 各叠加层的计数乘 `weight` 再相加。
    pub fn count(&self, weight: u32, read: impl Fn(&FrequencyLearner) -> u32) -> u32 {
        [&self.scene, &self.contact]
            .into_iter()
            .flatten()
            .fold(0, |sum, layer| {
                sum.saturating_add(weight.saturating_mul(read(layer)))
            })
    }

    pub fn write(&mut self, mut f: impl FnMut(&mut FrequencyLearner)) {
        for layer in [&mut self.scene, &mut self.contact].into_iter().flatten() {
            f(layer);
        }
    }

    pub fn flush(&mut self) {
        self.write(|layer| layer.flush());
    }
}
