//! 文件映射里的 Q8_0 矩阵：权重不拷进堆，直接把 `.qjm` 的映射切片当 `[BlockQ8_0]` 用，乘法交给 candle 的 Q8_0 内核
//! （`k_quants::matmul`，Apple 上走 NEON）。映射页是干净的文件页，不算进 iOS 键盘扩展被 jetsam 盯着的 dirty 内存。

use std::sync::Arc;

use candle_core::quantized::GgmlType;
use candle_core::quantized::k_quants::{self, BlockQ8_0};
use candle_core::{D, DType, Device, Result, Tensor};
use qingjian_format::Container;

/// Q8_0 一块的元素数与字节数（f16 尺度 + 32 个 int8）。
const BLOCK: usize = 32;
const BLOCK_BYTES: usize = 34;

/// `rows × cols` 的 Q8_0 矩阵（行优先，每行 `cols / 32` 块），数据在容器某一节的 `start` 起。
#[derive(Clone)]
pub(crate) struct MappedMatrix {
    file: Arc<Container>,

    tag: [u8; 4],

    /// 在那一节里的字节偏移。
    start: usize,

    rows: usize,

    cols: usize,

    device: Device,
}

impl MappedMatrix {
    /// 检查长度与对齐（`BlockQ8_0` 按 f16 两字节对齐）后建视图；不拷数据。
    pub(crate) fn new(
        file: Arc<Container>,
        tag: [u8; 4],
        start: usize,
        (rows, cols): (usize, usize),
        device: Device,
    ) -> std::result::Result<Self, crate::NeuralError> {
        if cols % BLOCK != 0 {
            return Err(crate::NeuralError::Corrupt(
                "Q8_0 row length is not a multiple of 32",
            ));
        }
        let len = rows * cols / BLOCK * BLOCK_BYTES;
        let section = file.bytes(tag)?;
        let bytes = section
            .get(start..start + len)
            .ok_or(crate::NeuralError::Corrupt(
                "Q8_0 tensor runs past its section",
            ))?;
        if bytes
            .as_ptr()
            .align_offset(std::mem::align_of::<BlockQ8_0>())
            != 0
        {
            return Err(crate::NeuralError::Corrupt("Q8_0 tensor is misaligned"));
        }
        Ok(Self {
            file,
            tag,
            start,
            rows,
            cols,
            device,
        })
    }

    fn blocks(&self) -> &[BlockQ8_0] {
        let section = self.file.bytes(self.tag).expect("checked in new");
        let count = self.rows * self.cols / BLOCK;
        let bytes = &section[self.start..self.start + count * BLOCK_BYTES];
        // SAFETY：BlockQ8_0 是 repr(C) 的 { f16, [i8; 32] }，34 字节、两字节对齐，任意位模式都合法；
        // 长度与对齐在 new 里查过，映射随 `file` 活着
        unsafe { std::slice::from_raw_parts(bytes.as_ptr().cast::<BlockQ8_0>(), count) }
    }

    /// `x` `[…, cols]`（f32）乘转置：`[…, rows]`，即 `x · Wᵀ`，与 `Linear` 同义。
    pub(crate) fn matmul(&self, x: &Tensor) -> Result<Tensor> {
        let mut dims = x.dims().to_vec();
        let k = x.dim(D::Minus1)?;
        if k != self.cols {
            candle_core::bail!("mapped matmul: input width {k}, expected {}", self.cols);
        }
        let m = x.elem_count() / k;
        let lhs = x.to_dtype(DType::F32)?.flatten_all()?.to_vec1::<f32>()?;
        let mut dst = vec![0f32; m * self.rows];
        k_quants::matmul((m, k, self.rows), &lhs, self.blocks(), &mut dst)?;
        *dims.last_mut().expect("has a last dim") = self.rows;
        Tensor::from_vec(dst, dims, &self.device)
    }

    /// 按行号查表：`ids` 任意形状（u32）→ 末尾加一维 `cols`，f32。
    pub(crate) fn embedding(&self, ids: &Tensor) -> Result<Tensor> {
        let mut dims = ids.dims().to_vec();
        let ids = ids.flatten_all()?.to_vec1::<u32>()?;
        let per_row = self.cols / BLOCK;
        let blocks = self.blocks();
        let mut out = vec![0f32; ids.len() * self.cols];
        for (row, &id) in out.chunks_mut(self.cols).zip(&ids) {
            let id = id as usize;
            if id >= self.rows {
                candle_core::bail!("mapped embedding: id {id} out of {} rows", self.rows);
            }
            BlockQ8_0::to_float(&blocks[id * per_row..(id + 1) * per_row], row);
        }
        dims.push(self.cols);
        Tensor::from_vec(out, dims, &self.device)
    }
}
