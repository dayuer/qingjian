use std::collections::HashMap;
use std::path::Path;

use serde::Deserialize;

use crate::NeuralError;

/// 未登录字符的 token。
pub const UNK: u32 = 1;

/// 句首 / 行分隔 token（训练时每行末尾补它，打分时当句首用）。
pub const EOS: u32 = 2;

/// 带上文的通变训练时上文最多这么长（`train_p2c.py --context` 的 `CTX_MAX`），推理侧对齐截断。
pub const CONTEXT_CHARS: usize = 32;

#[derive(Deserialize)]
struct VocabFile {
    tokens: Vec<String>,
}

/// 字级字表：一个 Unicode 字符一个 token，开头几个是 `<...>` 形式的特殊 token。
#[derive(Debug, Clone)]
pub struct Vocab {
    /// 字符 → token。
    index: HashMap<char, u32>,

    /// token → 字面，解码生成结果用。
    tokens: Vec<String>,

    /// P2C 字表里分开拼音与汉字两段的 token；字级 LM 的字表没有。
    sep: Option<u32>,
}

impl Vocab {
    pub fn load(path: &Path) -> Result<Self, NeuralError> {
        let text = std::fs::read_to_string(path).map_err(|source| NeuralError::Io {
            path: path.to_owned(),
            source,
        })?;
        Self::from_json(&text, path)
    }

    /// 解析 `vocab.json` 的正文；`path` 只用来报错。
    pub(crate) fn from_json(text: &str, path: &Path) -> Result<Self, NeuralError> {
        let file: VocabFile = serde_json::from_str(text).map_err(|source| NeuralError::Json {
            path: path.to_owned(),
            source,
        })?;
        // 特殊 token 靠「不是单个字符」认，不写死个数：字级 LM 的字表有 3 个，P2C 的多一个 `<sep>`。
        let mut index = HashMap::with_capacity(file.tokens.len());
        let mut sep = None;
        for (i, token) in file.tokens.iter().enumerate() {
            let mut chars = token.chars();
            match (chars.next(), chars.next()) {
                (Some(ch), None) => {
                    index.insert(ch, i as u32);
                }
                (Some(_), Some(_)) => {
                    if token == "<sep>" {
                        sep = Some(i as u32);
                    }
                }
                (None, _) => return Err(NeuralError::Corrupt("empty vocab token")),
            }
        }
        Ok(Self {
            index,
            tokens: file.tokens,
            sep,
        })
    }

    pub fn len(&self) -> usize {
        self.tokens.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// P2C 的 `<sep>`；不是 P2C 字表就没有。
    pub fn sep(&self) -> Option<u32> {
        self.sep
    }

    /// token 序列 → 文本；越界的 token 跳过。
    pub fn decode(&self, ids: &[u32]) -> String {
        ids.iter()
            .filter_map(|&id| self.tokens.get(id as usize))
            .map(String::as_str)
            .collect()
    }

    /// 逐字符编码，不认识的记 [`UNK`]。
    pub fn encode(&self, text: &str) -> Vec<u32> {
        text.chars()
            .map(|c| self.index.get(&c).copied().unwrap_or(UNK))
            .collect()
    }

    /// 光标前文 → token：**字表外的字符直接丢掉**，不记 [`UNK`]。
    /// 训练时的上文全是表内字（含标点的句子整句被剔了），喂 `<unk>` 是分布外的东西；
    /// 标点、罕见字丢掉更接近训练分布。再截到末 `max` 字——`max` 是模型自己声明的上限
    /// （[`CONTEXT_CHARS`]），传 0 就是不给前文。
    pub fn encode_context(&self, context: &str, max: usize) -> Vec<u32> {
        let mut ids: Vec<u32> = context
            .chars()
            .filter_map(|c| self.index.get(&c).copied())
            .collect();
        if ids.len() > max {
            ids.drain(..ids.len() - max);
        }
        ids
    }
}

#[cfg(test)]
mod tests {
    use super::{CONTEXT_CHARS, Vocab};

    fn vocab() -> Vocab {
        Vocab::from_json(
            r#"{"tokens": ["<pad>", "<unk>", "<eos>", "<sep>", "你", "好", "a", "1"]}"#,
            std::path::Path::new("test"),
        )
        .unwrap()
    }

    /// 表外字（标点、罕见字）直接丢，不留 `<unk>`：`,`、`啊`（不在造的字表里）都不该冒出 token。
    #[test]
    fn context_drops_characters_outside_the_vocabulary() {
        let ids = vocab().encode_context("你好，啊a1", CONTEXT_CHARS);
        assert_eq!(ids, vec![4, 5, 6, 7], "只剩 你 好 a 1：{ids:?}");
    }

    /// 超过 `CONTEXT_CHARS` 只留末段，且丢字发生在截断之前——末尾的表外字不能把前面挤掉。
    #[test]
    fn context_keeps_the_tail_after_dropping() {
        let long = "你".repeat(CONTEXT_CHARS + 5) + "，";
        assert_eq!(
            vocab().encode_context(&long, CONTEXT_CHARS).len(),
            CONTEXT_CHARS
        );
    }

    /// `max` 为 0（老模型没声明前文）时一个字都不给。
    #[test]
    fn context_can_be_disabled() {
        assert!(vocab().encode_context("你好", 0).is_empty());
    }
}
