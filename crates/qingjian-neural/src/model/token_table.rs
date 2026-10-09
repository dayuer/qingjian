//! 字嵌入表：输入查表与输出层共用同一份权重（共享嵌入）。8 位时两边读同一块映射，不各存一份。

use candle_core::{Module, Result, Tensor};
use candle_nn::{Embedding, Linear};

use crate::mapped_matrix::MappedMatrix;

pub(super) enum TokenTable {
    Dense {
        embedding: Embedding,

        /// 输出层：权重与 `embedding` 是同一个张量（candle 的张量按引用计数共享存储）。
        head: Linear,
    },

    Mapped(MappedMatrix),
}

impl TokenTable {
    /// `idx` `[b, t]` → `[b, t, n_embd]`。
    pub(super) fn embed(&self, idx: &Tensor) -> Result<Tensor> {
        match self {
            Self::Dense { embedding, .. } => embedding.forward(idx),
            Self::Mapped(table) => table.embedding(idx),
        }
    }

    /// `[b, t, n_embd]` → logits `[b, t, vocab]`。
    pub(super) fn logits(&self, x: &Tensor) -> Result<Tensor> {
        match self {
            Self::Dense { head, .. } => head.forward(x),
            Self::Mapped(table) => table.matmul(x),
        }
    }
}
