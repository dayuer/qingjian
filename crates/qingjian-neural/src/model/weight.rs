//! 线性层的权重：稠密（f32 / f16，`candle_nn::Linear`），或映射在 `.qjm` 里的 Q8_0（`MappedMatrix` + 偏置）。

use candle_core::{Module, Result, Tensor};
use candle_nn::Linear;

use crate::mapped_matrix::MappedMatrix;

pub(super) enum Weight {
    Dense(Linear),

    /// 8 位：权重在文件映射里，乘法时现算，不展开成浮点矩阵。
    Mapped {
        matrix: MappedMatrix,

        bias: Option<Tensor>,
    },
}

impl Module for Weight {
    fn forward(&self, x: &Tensor) -> Result<Tensor> {
        match self {
            Self::Dense(linear) => linear.forward(x),
            Self::Mapped { matrix, bias } => {
                let y = matrix.matmul(x)?;
                match bias {
                    Some(bias) => y.broadcast_add(bias),
                    None => Ok(y),
                }
            }
        }
    }
}
