//! 线性层的权重：稠密（f32 / f16，`candle_nn::Linear`），或 8 位常驻（Q8_0 的 `QMatMul` + 偏置）。

use candle_core::quantized::QMatMul;
use candle_core::{Module, Result, Tensor};
use candle_nn::Linear;

pub(super) enum Weight {
    Dense(Linear),

    /// 权重以 Q8_0 块常驻（每 32 个一组 f16 尺度 + int8），乘法时现算，不展开成浮点矩阵。
    Quantized {
        matmul: QMatMul,

        bias: Option<Tensor>,
    },
}

impl Module for Weight {
    fn forward(&self, x: &Tensor) -> Result<Tensor> {
        match self {
            Self::Dense(linear) => linear.forward(x),
            Self::Quantized { matmul, bias } => {
                let y = matmul.forward(x)?;
                match bias {
                    Some(bias) => y.broadcast_add(bias),
                    None => Ok(y),
                }
            }
        }
    }
}
