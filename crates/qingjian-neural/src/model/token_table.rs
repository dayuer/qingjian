//! 字嵌入表：输入查表与输出层共用同一份权重（共享嵌入）。8 位时两边共用同一份 Q8_0 张量，不各存一份。

use candle_core::quantized::QMatMul;
use candle_core::{Module, Result, Tensor};
use candle_nn::{Embedding, Linear};

pub(super) enum TokenTable {
    Dense {
        embedding: Embedding,

        /// 输出层：权重与 `embedding` 是同一个张量（candle 的张量按引用计数共享存储）。
        head: Linear,
    },

    Quantized(QMatMul),
}

impl TokenTable {
    /// `idx` `[b, t]` → `[b, t, n_embd]`。
    pub(super) fn embed(&self, idx: &Tensor) -> Result<Tensor> {
        match self {
            Self::Dense { embedding, .. } => embedding.forward(idx),
            Self::Quantized(table) => table.embedding(idx),
        }
    }

    /// `[b, t, n_embd]` → logits `[b, t, vocab]`。
    pub(super) fn logits(&self, x: &Tensor) -> Result<Tensor> {
        match self {
            Self::Dense { head, .. } => head.forward(x),
            Self::Quantized(table) => table.forward(x),
        }
    }
}
