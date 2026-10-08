//! 8 位权重：打包时把 safetensors 里的线性层与字嵌入表量化成 Q8_0，连同其余张量（f32）写成一段 GGUF；
//! 加载时线性层与嵌入表保持 Q8_0 常驻（见 `model::Weight` / `model::TokenTable`），LayerNorm、偏置、位置嵌入解成 f32。
//! 精度验收见 `docs/notes/neural-rescoring.md`「8 位量化」一节。

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;

use candle_core::quantized::{GgmlDType, QMatMul, QTensor, gguf_file};
use candle_core::safetensors::SliceSafetensors;
use candle_core::{DType, Device, Tensor};

use crate::NeuralError;

/// 量化成 Q8_0 的张量（按名字后缀认）：四种线性层与共享的字嵌入表。
const QUANTIZED: [&str; 5] = [
    "attn.qkv.weight",
    "attn.proj.weight",
    "mlp.fc.weight",
    "mlp.proj.weight",
    "tok_emb.weight",
];

/// 一段 GGUF 读出来的全部张量，按名字取走。
pub struct QuantizedWeights {
    tensors: HashMap<String, QTensor>,
}

impl QuantizedWeights {
    /// safetensors 原文 → GGUF 字节：[`QUANTIZED`] 里的张量 Q8_0，其余 f32。
    pub fn encode(safetensors: &[u8]) -> Result<Vec<u8>, NeuralError> {
        let source = SliceSafetensors::new(safetensors)?;
        let mut tensors = Vec::new();
        for (name, _) in source.tensors() {
            let tensor = source.load(&name, &Device::Cpu)?.to_dtype(DType::F32)?;
            let dtype = if QUANTIZED.iter().any(|suffix| name.ends_with(suffix)) {
                GgmlDType::Q8_0
            } else {
                GgmlDType::F32
            };
            tensors.push((name, QTensor::quantize(&tensor, dtype)?));
        }
        tensors.sort_by(|a, b| a.0.cmp(&b.0));
        let refs: Vec<(&str, &QTensor)> = tensors.iter().map(|(n, t)| (n.as_str(), t)).collect();
        let mut out = Cursor::new(Vec::new());
        gguf_file::write(&mut out, &[], &refs)?;
        Ok(out.into_inner())
    }

    /// GGUF 字节 → 全部张量（拷进 candle 自己的存储，之后不再引用 `bytes`）。
    pub fn read(bytes: &[u8], device: &Device) -> Result<Self, NeuralError> {
        let mut reader = Cursor::new(bytes);
        let content = gguf_file::Content::read(&mut reader)?;
        let mut tensors = HashMap::with_capacity(content.tensor_infos.len());
        for name in content.tensor_infos.keys() {
            let tensor = content.tensor(&mut reader, name, device)?;
            tensors.insert(name.clone(), tensor);
        }
        Ok(Self { tensors })
    }

    fn take(&mut self, name: &str) -> Result<QTensor, NeuralError> {
        self.tensors
            .remove(name)
            .ok_or(NeuralError::Corrupt("quantized weights miss a tensor"))
    }

    /// 8 位常驻的矩阵；张量本身不是 Q8_0（老打包或手工改过）时报错，免得悄悄展开成浮点。
    pub fn matmul(&mut self, name: &str) -> Result<QMatMul, NeuralError> {
        let tensor = self.take(name)?;
        if tensor.dtype() != GgmlDType::Q8_0 {
            return Err(NeuralError::Corrupt("expected a Q8_0 tensor"));
        }
        Ok(QMatMul::from_arc(Arc::new(tensor))?)
    }

    /// 解成 f32 的小张量（LayerNorm、偏置、位置嵌入）。
    pub fn dense(&mut self, name: &str) -> Result<Tensor, NeuralError> {
        let tensor = self.take(name)?;
        Ok(tensor.dequantize(&tensor.device())?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 编码再读回：线性层与嵌入表是 Q8_0（解出来误差在 1% 以内），别的张量 f32 原样；缺张量、类型不对都报错。
    #[test]
    fn encode_and_read_back() {
        let device = Device::Cpu;
        let weight = Tensor::randn(0f32, 1.0, (64, 64), &device).unwrap();
        let bias = Tensor::randn(0f32, 1.0, 64, &device).unwrap();
        let dir = std::env::temp_dir().join("qingjian-neural-tests/encode");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("w.safetensors");
        candle_core::safetensors::save(
            &HashMap::from([
                (
                    "blocks.0.mlp.fc.weight".to_owned(),
                    weight.to_dtype(DType::F16).unwrap(),
                ),
                ("blocks.0.mlp.fc.bias".to_owned(), bias.clone()),
            ]),
            &file,
        )
        .unwrap();
        let bytes = QuantizedWeights::encode(&std::fs::read(&file).unwrap()).unwrap();
        let mut weights = QuantizedWeights::read(&bytes, &device).unwrap();

        assert!(matches!(
            weights.matmul("blocks.0.mlp.fc.bias"),
            Err(NeuralError::Corrupt(_))
        ));
        let mut again = QuantizedWeights::read(&bytes, &device).unwrap();
        let restored = again.dense("blocks.0.mlp.fc.bias").unwrap();
        let diff = (restored - &bias)
            .unwrap()
            .abs()
            .unwrap()
            .max_all()
            .unwrap();
        assert_eq!(diff.to_scalar::<f32>().unwrap(), 0.0);

        let matmul = weights.matmul("blocks.0.mlp.fc.weight").unwrap();
        assert!(matches!(matmul, QMatMul::QTensor(_)));
        let dense = matmul
            .dequantize_f16()
            .unwrap()
            .to_dtype(DType::F32)
            .unwrap();
        let error = (dense - &weight)
            .unwrap()
            .sqr()
            .unwrap()
            .sum_all()
            .unwrap()
            .sqrt()
            .unwrap();
        let norm = weight.sqr().unwrap().sum_all().unwrap().sqrt().unwrap();
        let relative = error.to_scalar::<f32>().unwrap() / norm.to_scalar::<f32>().unwrap();
        assert!(relative < 0.01, "{relative}");
        assert!(weights.dense("missing").is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
