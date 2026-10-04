//! 当前生效的叠加层。恋爱场景开场景层（选了对象再开对象层），写只进叠加层；
//! 日常与工作只在选了对象时开对象层（不开场景层），写时全局与对象层都写。

use std::path::Path;

use qingjian_cloud_proto::Scene;
use qingjian_core::Learner;
use qingjian_learning::FrequencyLearner;

use super::{contact_learning_dir, is_contact_id, load_layer, scene_learning_dir};

#[derive(Debug, Default)]
pub struct Overlay {
    scene: Option<FrequencyLearner>,

    contact: Option<FrequencyLearner>,

    /// 写只进叠加层、不碰全局（恋爱场景）。
    exclusive: bool,
}

impl Overlay {
    /// 给了合格的对象 id、且对象目录还在时才开对象层。
    /// 对象目录只由建对象时创建，这里不建：忘掉的人不会因为键盘还选着它而被重新建出来。
    pub fn open(memory_dir: &Path, scene: Scene, contact: Option<&str>) -> Self {
        let contact = contact
            .filter(|id| is_contact_id(id) && memory_dir.join(id).is_dir())
            .map(|id| load_layer(&contact_learning_dir(memory_dir, id)));
        if scene != Scene::Dating {
            return Self {
                scene: None,
                contact,
                exclusive: false,
            };
        }
        Self {
            scene: Some(load_layer(&scene_learning_dir(memory_dir, scene))),
            contact,
            exclusive: true,
        }
    }

    /// 写只进叠加层（恋爱场景）；为假时全局也写。
    pub fn exclusive(&self) -> bool {
        self.exclusive
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
