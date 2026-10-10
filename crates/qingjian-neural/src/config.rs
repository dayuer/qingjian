use std::path::Path;

use serde::Deserialize;

use crate::NeuralError;

/// 模型结构，来自导出目录的 `config.json`（训练脚本 `common.py::ModelConfig` 原样写出）。
#[derive(Debug, Clone, Deserialize)]
pub struct ModelConfig {
    /// 字表大小。
    pub vocab_size: usize,

    /// 层数。
    pub n_layer: usize,

    /// 隐层宽度。
    pub n_embd: usize,

    /// 注意力头数。
    pub n_head: usize,

    /// 最长上下文（token 数），位置嵌入的行数。
    pub context: usize,

    /// **带前文训练**的模型才有：上文最长多少字（`train_p2c.py` 的 `CTX_MAX`）。
    /// 老模型（通变）没有这个字段，推理侧据此一律不喂前文——它训练时没见过，喂了读不懂反而掉分。
    #[serde(default)]
    pub context_chars: Option<usize>,
}

impl ModelConfig {
    /// 解析 `config.json` 的正文；`path` 只用来报错。
    pub(crate) fn from_json(text: &str, path: &Path) -> Result<Self, NeuralError> {
        serde_json::from_str(text).map_err(|source| NeuralError::Json {
            path: path.to_owned(),
            source,
        })
    }
}
