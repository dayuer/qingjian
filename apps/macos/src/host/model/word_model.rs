//! 知微（含章·知微）在壳里的加载：`[model] scorers = "both"` 时与通变一起在后台加载，接到 Engine 的词级重排上；
//! 配置切回 `tongbian` 时真卸掉（内存与日志对比要准）。素笺分叉，见 cloud/docs/fork-patch.md。

use std::sync::mpsc::{TryRecvError, channel};

use qingjian_core::sentence::SentenceScorer;
use qingjian_neural::{CharScorer, NeuralError};
use qingjian_platform::ScorerSet;

use crate::app::paths;
use crate::host::Host;

impl Host {
    /// 后台加载知微并预热。配置不要就卸掉已有的；没有文件或已经在加载 / 已接上就什么都不做。
    pub(super) fn load_word_model(&mut self) {
        if self.settings.config().model.scorers != ScorerSet::Both {
            self.unload_word_model();
            return;
        }
        if self.word_loader.is_some() || self.engine.has_word_scorer() {
            return;
        }
        let Some(path) = paths::model_path() else {
            tracing::info!("没有知微模型文件，不按前文排词");
            return;
        };
        let (tx, rx) = channel::<Result<Box<dyn SentenceScorer>, NeuralError>>();
        let spawned = std::thread::Builder::new()
            .name("qingjian-word-model-load".to_owned())
            .spawn(move || {
                let started = std::time::Instant::now();
                let loaded = CharScorer::load(&path).and_then(|scorer| {
                    scorer.score("今天", &["的"])?;
                    Ok(Box::new(scorer) as Box<dyn SentenceScorer>)
                });
                if loaded.is_ok() {
                    tracing::info!(
                        path = %path.display(),
                        total_ms = started.elapsed().as_millis(),
                        "知微已加载并预热"
                    );
                }
                let _ = tx.send(loaded);
            });
        match spawned {
            Ok(_) => {
                self.word_loader = Some(rx);
                self.rescore.watch_loading();
            }
            Err(error) => tracing::warn!(%error, "起不了知微加载线程，不按前文排词"),
        }
    }

    /// 知微加载有结果了就接上。返回是否还在加载。
    pub(super) fn attach_loaded_word_model(&mut self) -> bool {
        let Some(rx) = &self.word_loader else {
            return false;
        };
        match rx.try_recv() {
            Ok(Ok(scorer)) => {
                self.engine.set_async_word_scorer(Some(scorer));
                self.word_loader = None;
                // 知微上线时正在组句：这一轮的查询没见过它，补查一次攒下要打分的词（同通变接上时的做法）
                self.rescore_current_round();
                false
            }
            Ok(Err(error)) => {
                tracing::warn!(%error, "知微加载失败，不按前文排词");
                self.word_loader = None;
                false
            }
            Err(TryRecvError::Empty) => true,
            Err(TryRecvError::Disconnected) => {
                self.word_loader = None;
                false
            }
        }
    }

    /// 卸掉知微（配置切回 tongbian 或关掉本地模型）。
    pub(super) fn unload_word_model(&mut self) {
        if self.word_loader.take().is_some() || self.engine.has_word_scorer() {
            tracing::info!("知微已卸载");
        }
        self.engine.set_async_word_scorer(None);
    }
}
