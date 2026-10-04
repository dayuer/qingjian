//! 本地续写：知微贪心解码，接着前文往下写几个字，给 Tab 接受用。
//! 素笺分叉，见 cloud/docs/specs/2026-10-04-context-prediction-design.md。

use candle_core::Tensor;

use crate::vocab::EOS;
use crate::{CharScorer, NeuralError};

/// 续写遇到这些就停：句读标点与换行，再写就是下一句了。
pub(crate) const STOPS: &[char] = &[
    '，', '。', '！', '？', '；', '：', '、', '\n', ',', '.', '!', '?', ';',
];

impl CharScorer {
    /// 接着 `before` 续写最多 `max_chars` 个字（贪心），返回续写文本与每字平均 log 概率；
    /// `before` 空白、`max_chars` 为 0、或第一个字就是标点 / 句尾时返回 `None`。
    /// 前文过长从左截到模型上下文给续写留出位置；序列开头保留 `<eos>`（训练时每行末尾补它，当句首用）。
    pub fn continue_text(
        &self,
        before: &str,
        max_chars: usize,
    ) -> Result<Option<(String, f64)>, NeuralError> {
        if before.trim().is_empty() || max_chars == 0 {
            return Ok(None);
        }
        let limit = self.model().config().context;
        let mut ids: Vec<u32> = self.vocab().encode(before);
        let room = limit.saturating_sub(max_chars + 1).max(1);
        if ids.len() > room {
            ids.drain(..ids.len() - room);
        }
        ids.insert(0, EOS);
        let device = self.model().device();
        let split = ids.len() - 1;
        let mut cache = self.model().prefix_cache(&ids[..split])?;
        let mut last = Tensor::from_vec(vec![ids[split]], (1, 1), device)?;
        let mut out: Vec<u32> = Vec::new();
        let mut total = 0.0f64;
        for _ in 0..max_chars {
            let (log_probs, grown) = self.model().step(&cache, &last)?;
            let row = log_probs.squeeze(0)?.to_vec1::<f32>()?;
            let Some((token, lp)) = row
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.total_cmp(b.1))
                .map(|(token, lp)| (token as u32, f64::from(*lp)))
            else {
                break;
            };
            let text = self.vocab().decode(&[token]);
            if token == EOS || text.starts_with('<') || text.chars().any(|c| STOPS.contains(&c)) {
                break;
            }
            out.push(token);
            total += lp;
            cache = grown;
            last = Tensor::from_vec(vec![token], (1, 1), device)?;
        }
        if out.is_empty() {
            return Ok(None);
        }
        let text = self.vocab().decode(&out);
        Ok(Some((text, total / out.len() as f64)))
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::CharScorer;

    fn scorer() -> Option<CharScorer> {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/models/hanzhang-zhiwei");
        dir.exists().then(|| CharScorer::load(&dir).unwrap())
    }

    #[test]
    fn continues_a_few_characters_and_stops_at_punctuation() {
        let Some(scorer) = scorer() else {
            eprintln!("没有知微模型，跳过");
            return;
        };
        let (text, average) = scorer
            .continue_text("今天下午我们开会讨论输入法的", 8)
            .unwrap()
            .expect("有续写");
        eprintln!("续写：{text}（{average:.2}）");
        assert!(!text.is_empty() && text.chars().count() <= 8, "{text}");
        assert!(text.chars().all(|c| !super::STOPS.contains(&c)), "{text}");
        assert!(average <= 0.0 && average > -20.0, "{average}");
        let (one, _) = scorer
            .continue_text("今天下午我们开会讨论输入法的", 1)
            .unwrap()
            .unwrap();
        assert_eq!(one.chars().count(), 1);
        assert_eq!(scorer.continue_text("", 8).unwrap(), None);
        assert_eq!(scorer.continue_text("   ", 8).unwrap(), None);
        assert_eq!(scorer.continue_text("今天", 0).unwrap(), None);
        let long = "很长的前文。".repeat(40);
        assert!(scorer.continue_text(&long, 4).is_ok());
    }
}
