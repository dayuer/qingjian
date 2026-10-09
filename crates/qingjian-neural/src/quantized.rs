//! 8 位权重：打包时把 safetensors 里的线性层与字嵌入表量化成 Q8_0，连同其余张量（f32）写成一段 GGUF；
//! 加载时不拷：线性层与嵌入表是 `.qjm` 映射上的 Q8_0 视图（见 `MappedMatrix`），只有 LayerNorm、偏置、位置嵌入拷成 f32。
//! 精度验收见 `docs/notes/neural-rescoring.md`「8 位量化」一节。

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;

use candle_core::quantized::gguf_file::TensorInfo;
use candle_core::quantized::{GgmlDType, QTensor, gguf_file};
use candle_core::safetensors::SliceSafetensors;
use candle_core::{DType, Device, Tensor};
use qingjian_format::Container;

use crate::NeuralError;
use crate::mapped_matrix::MappedMatrix;
use crate::qjm::QUANTIZED_TAG;

/// 量化成 Q8_0 的张量（按名字后缀认）：四种线性层与共享的字嵌入表。
const QUANTIZED: [&str; 5] = [
    "attn.qkv.weight",
    "attn.proj.weight",
    "mlp.fc.weight",
    "mlp.proj.weight",
    "tok_emb.weight",
];

/// 映射着的 8 位那一节：张量按名字取走，Q8_0 的给映射视图（[`MappedMatrix`]），f32 的拷出来。
pub struct QuantizedWeights {
    file: Arc<Container>,

    /// 张量数据区在那一节里的起点。
    base: usize,

    infos: HashMap<String, TensorInfo>,

    device: Device,
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

    /// 映射容器里的 8 位那一节：只解析 GGUF 头（张量名、形状、偏移），数据留在映射里。
    pub fn map(file: Arc<Container>, device: Device) -> Result<Self, NeuralError> {
        let bytes = file.bytes(QUANTIZED_TAG)?;
        let content = gguf_file::Content::read(&mut Cursor::new(bytes))?;
        Ok(Self {
            base: usize::try_from(content.tensor_data_offset)
                .map_err(|_| NeuralError::Corrupt("GGUF data offset overflows"))?,
            infos: content.tensor_infos,
            file,
            device,
        })
    }

    fn take(&mut self, name: &str) -> Result<TensorInfo, NeuralError> {
        self.infos
            .remove(name)
            .ok_or(NeuralError::Corrupt("quantized weights miss a tensor"))
    }

    fn start(&self, info: &TensorInfo) -> Result<usize, NeuralError> {
        usize::try_from(info.offset)
            .ok()
            .and_then(|offset| offset.checked_add(self.base))
            .ok_or(NeuralError::Corrupt("GGUF tensor offset overflows"))
    }

    /// Q8_0 的二维矩阵，映射视图；不是 Q8_0（老打包或手工改过）时报错，免得悄悄当成别的格式读。
    pub(crate) fn matrix(&mut self, name: &str) -> Result<MappedMatrix, NeuralError> {
        let info = self.take(name)?;
        if info.ggml_dtype != GgmlDType::Q8_0 {
            return Err(NeuralError::Corrupt("expected a Q8_0 tensor"));
        }
        let shape = info.shape.dims2()?;
        MappedMatrix::new(
            self.file.clone(),
            QUANTIZED_TAG,
            self.start(&info)?,
            shape,
            self.device.clone(),
        )
    }

    /// f32 的小张量（LayerNorm、偏置、位置嵌入），拷一份出来（合计不到 1MB）。
    pub(crate) fn dense(&mut self, name: &str) -> Result<Tensor, NeuralError> {
        let info = self.take(name)?;
        if info.ggml_dtype != GgmlDType::F32 {
            return Err(NeuralError::Corrupt("expected an f32 tensor"));
        }
        let start = self.start(&info)?;
        let len = info.shape.elem_count() * 4;
        let bytes = self
            .file
            .bytes(QUANTIZED_TAG)?
            .get(start..start + len)
            .ok_or(NeuralError::Corrupt("f32 tensor runs past its section"))?;
        let values: Vec<f32> = bytes
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect();
        Ok(Tensor::from_vec(values, info.shape.clone(), &self.device)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use candle_core::Module;
    use candle_core::quantized::QMatMul;
    use qingjian_format::{Kind, Metadata, Writer};

    /// 编码、打进容器、映射回来：映射视图的乘法与查表和 candle 自己读进内存的 QMatMul 对拍（< 1e-3），
    /// 与原矩阵的相对误差 < 1%；f32 小张量原样；类型不对、缺张量都报错。
    #[test]
    fn mapped_weights_match_candle_qmatmul() {
        let device = Device::Cpu;
        let weight = Tensor::randn(0f32, 1.0, (96, 64), &device).unwrap();
        let bias = Tensor::randn(0f32, 1.0, 96, &device).unwrap();
        let dir = std::env::temp_dir().join("qingjian-neural-tests/mapped");
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
        let gguf = QuantizedWeights::encode(&std::fs::read(&file).unwrap()).unwrap();
        let packed = dir.join("w.qjm");
        Writer::new(Kind::Model, &Metadata::default())
            .unwrap()
            .section(QUANTIZED_TAG, &gguf)
            .write_to(&packed)
            .unwrap();
        let container = Arc::new(Container::open(&packed, Kind::Model).unwrap());
        let mut weights = QuantizedWeights::map(container, device.clone()).unwrap();

        let mut reader = Cursor::new(gguf.as_slice());
        let content = gguf_file::Content::read(&mut reader).unwrap();
        let reference = QMatMul::from_qtensor(
            content
                .tensor(&mut reader, "blocks.0.mlp.fc.weight", &device)
                .unwrap(),
        )
        .unwrap();

        assert!(matches!(
            weights.matrix("blocks.0.mlp.fc.bias"),
            Err(NeuralError::Corrupt(_))
        ));
        let mut again = QuantizedWeights::map(
            Arc::new(Container::open(&packed, Kind::Model).unwrap()),
            device.clone(),
        )
        .unwrap();
        let restored = again.dense("blocks.0.mlp.fc.bias").unwrap();
        let diff = (restored - &bias)
            .unwrap()
            .abs()
            .unwrap()
            .max_all()
            .unwrap();
        assert_eq!(diff.to_scalar::<f32>().unwrap(), 0.0);
        assert!(again.dense("blocks.0.mlp.fc.weight").is_err());

        let mapped = weights.matrix("blocks.0.mlp.fc.weight").unwrap();
        let x = Tensor::randn(0f32, 1.0, (2, 3, 64), &device).unwrap();
        let ours = mapped.matmul(&x).unwrap();
        let theirs = reference.forward(&x).unwrap();
        assert_eq!(ours.dims(), &[2, 3, 96]);
        let gap = (&ours - &theirs).unwrap().abs().unwrap().max_all().unwrap();
        assert!(gap.to_scalar::<f32>().unwrap() < 1e-3);

        let ids = Tensor::new(&[[0u32, 95, 7]], &device).unwrap();
        let rows = mapped.embedding(&ids).unwrap();
        let expected = reference.embedding(&ids).unwrap();
        let gap = (rows - expected).unwrap().abs().unwrap().max_all().unwrap();
        assert!(gap.to_scalar::<f32>().unwrap() < 1e-6);

        let dense = mapped
            .embedding(&Tensor::arange(0u32, 96, &device).unwrap())
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
