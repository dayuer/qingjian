use qingjian_core::sentence::SentenceScorer;

use crate::{CharScorer, P2c};

/// 字级模型：条件是光标前文，忽略按键。
impl SentenceScorer for CharScorer {
    fn continue_text(&self, before: &str, max_chars: usize) -> Option<(String, f64)> {
        match CharScorer::continue_text(self, before, max_chars) {
            Ok(result) => result,
            Err(error) => {
                tracing::warn!(%error, "本地续写失败，本次不用");
                None
            }
        }
    }

    fn score(&self, context: &str, _keys: &str, texts: &[&str]) -> Vec<f64> {
        match CharScorer::score(self, context, texts) {
            Ok(scores) => scores,
            Err(error) => {
                tracing::warn!(%error, "神经重打分失败，本次不用");
                Vec::new()
            }
        }
    }
}

/// P2C 模型：条件是用户敲的那段按键 + 光标前文。
///
/// 冻结集 8322 句上比字级模型的重排高 0.69 个点（41.80% → 42.49%，配对 McNemar p=0.003），
/// 延迟也低（中位 18.5 对 22.0 ms），而且同一个模型还能造词——所以整句重排换成它。
/// 留出验证过 λ 没有过拟合：0.5 / 0.75 / 1.0 在两半上都赢，只有 0.25 会输。
/// 前文按 `[eos] 前文 拼音 <sep>` 喂进去；老模型（训练时没带前文）读不懂这段前文，
/// 会退化成乱读，所以带前文的模型只配合带上文重训的那版（见 `docs/notes/p2c-context-training.md`）。
pub struct P2cScorer(pub CharScorer);

impl P2cScorer {
    /// 字表里没有 `<sep>` 的不是 P2C 模型，建不出来。
    pub fn new(scorer: CharScorer) -> Option<Self> {
        scorer.vocab().sep()?;
        Some(Self(scorer))
    }
}

impl SentenceScorer for P2cScorer {
    fn score(&self, context: &str, keys: &str, texts: &[&str]) -> Vec<f64> {
        match self.0.score_p2c(context, keys, texts) {
            Ok(scores) => scores,
            Err(error) => {
                tracing::warn!(%error, "P2C 重打分失败，本次不用");
                Vec::new()
            }
        }
    }

    fn generate(&self, keys: &str, beam: usize, max_chars: usize) -> Vec<String> {
        // `SentenceScorer::generate` 没有上文参数，生成这条路先不给前文
        let Some(decoder) = P2c::new(self.0.model(), self.0.vocab()) else {
            return Vec::new();
        };
        match decoder.convert("", keys, beam, max_chars) {
            Ok(items) => items.into_iter().map(|item| item.text).collect(),
            Err(error) => {
                tracing::warn!(%error, "P2C 生成失败，本次不用");
                Vec::new()
            }
        }
    }
}
