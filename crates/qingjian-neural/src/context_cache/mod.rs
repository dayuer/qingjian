//! 一段前缀（光标前文或通变的拼音）的 K / V 跨调用复用：完全相同直接用，新序列与旧序列有公共前缀就截到公共前缀、
//! 只算多出来的 token，没有公共前缀才整段重算。对应参考实现的 `DecoderOnlyNNModel::PrefillContext`，
//! 规则与取舍见 docs/notes/ref-ime-impl/b4-b6-context-cache-and-tiers.md 的 B4 节。

#[cfg(test)]
mod tests;

use candle_core::Result;

use crate::model::{CharLm, PrefixCache};

/// 缓存最多占这么多字节，超了用完就丢（每次现算）。8 层 / 448 宽 f32 的 65 个 token 约 1.8 MB，正常碰不到；
/// 是给更大的模型或更长的上下文兜底的，iOS 键盘扩展的内存余量见 `KeyboardModel.guardMemoryPressure`。
pub(crate) const CONTEXT_CACHE_MAX_BYTES: usize = 4 << 20;

/// 一格前缀缓存：token 序列与它的 K / V。
pub(crate) struct ContextCache {
    ids: Vec<u32>,

    /// `None` 是还没算过或被清掉了。
    cache: Option<PrefixCache>,

    /// 超过这个字节数用完就丢。
    max_bytes: usize,

    /// 最近一次 [`Self::sync`] 复用的 token 数与前缀总长（真机日志看命中率）。
    last_reuse: (usize, usize),
}

impl ContextCache {
    pub(crate) fn new(max_bytes: usize) -> Self {
        Self {
            ids: Vec::new(),
            cache: None,
            max_bytes,
            last_reuse: (0, 0),
        }
    }

    /// 让缓存对应 `ids`，返回对应的 K / V（克隆只复制句柄）。复用了多少 token 记在 [`Self::last_reuse`]。
    pub(crate) fn sync(&mut self, model: &CharLm, ids: &[u32]) -> Result<PrefixCache> {
        let reused = match &self.cache {
            Some(cache) if self.ids == ids => {
                self.last_reuse = (ids.len(), ids.len());
                return Ok(cache.clone());
            }
            Some(cache) => {
                let common = common_prefix_len(&self.ids, ids);
                if common == 0 {
                    None
                } else {
                    let kept = if common < cache.len() {
                        cache.truncate(common)?
                    } else {
                        cache.clone()
                    };
                    Some((common, model.extend_cache(&kept, &ids[common..])?))
                }
            }
            None => None,
        };
        let (common, cache) = match reused {
            Some(found) => found,
            None => (0, model.prefix_cache(ids)?),
        };
        self.last_reuse = (common, ids.len());
        self.ids.clear();
        self.ids.extend_from_slice(ids);
        self.cache = Some(cache.clone());
        Ok(cache)
    }

    /// 用完之后调：超过字节上限就不留。
    pub(crate) fn settle(&mut self) {
        if self
            .cache
            .as_ref()
            .is_some_and(|c| c.bytes() > self.max_bytes)
        {
            self.clear();
        }
    }

    /// 丢掉缓存（私密输入、换学习范围时调，前文的状态不在内存里留着）。
    pub(crate) fn clear(&mut self) {
        self.ids.clear();
        self.cache = None;
    }

    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool {
        self.cache.is_none()
    }

    /// 最近一次 [`Self::sync`] 的（复用 token 数, 前缀总长）。
    pub(crate) fn last_reuse(&self) -> (usize, usize) {
        self.last_reuse
    }
}

fn common_prefix_len(a: &[u32], b: &[u32]) -> usize {
    a.iter().zip(b).take_while(|(x, y)| x == y).count()
}
