//! 四项功能各自开没开（同意记录）；新用户全关。

use serde::{Deserialize, Serialize};

use crate::Feature;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Consents {
    pub clipboard: bool,

    pub sync: bool,

    pub input_log: bool,

    pub llm: bool,
}

impl Consents {
    pub fn get(&self, feature: Feature) -> bool {
        match feature {
            Feature::Clipboard => self.clipboard,
            Feature::Sync => self.sync,
            Feature::InputLog => self.input_log,
            Feature::Llm => self.llm,
        }
    }

    pub fn set(&mut self, feature: Feature, enabled: bool) {
        match feature {
            Feature::Clipboard => self.clipboard = enabled,
            Feature::Sync => self.sync = enabled,
            Feature::InputLog => self.input_log = enabled,
            Feature::Llm => self.llm = enabled,
        }
    }
}
