//! 本地神经整句模型：后台线程加载（第一次前向要预热，几百毫秒），channel 交接，
//! 会话的轮询入口无锁 `try_recv`，接上即生效。照的是 Mac 壳 `host/model` 的模式。
//!
//! 缺省 CPU 推理（qingjian-neural 不带特性）：模型只有 8 层 / 448 隐宽，重打分又是停键后的异步路径，
//! 不差这几毫秒；键盘扩展进程里少链一个 Metal 依赖也少一分被系统收拾的风险。真机量过延迟再考虑开 metal。

use std::path::Path;
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use qingjian_core::sentence::SentenceScorer;
use qingjian_neural::{CharScorer, NeuralError, P2cScorer};

/// `qj_model_state` 的取值：未加载。
pub const MODEL_IDLE: u8 = 0;

/// `qj_model_state` 的取值：加载中（查询照常，重打分还没生效）。
pub const MODEL_LOADING: u8 = 1;

/// `qj_model_state` 的取值：在用（`Engine` 的异步重打分已接上）。
pub const MODEL_ACTIVE: u8 = 2;

/// `qj_model_state` 的取值：上次加载失败。
pub const MODEL_FAILED: u8 = 3;

/// 模型的加载状态与交接信道。
pub struct ModelState {
    state: u8,

    /// 加载线程的结果信道；`None` 是没在加载。
    loader: Option<Receiver<Result<Box<dyn SentenceScorer>, NeuralError>>>,
}

impl Default for ModelState {
    fn default() -> Self {
        Self {
            state: MODEL_IDLE,
            loader: None,
        }
    }
}

impl ModelState {
    pub fn state(&self) -> u8 {
        self.state
    }

    /// 开始异步加载。已在加载或在用时返回 `false`；起不了线程按失败记。
    pub fn load(&mut self, path: &Path, p2c: bool) -> bool {
        if self.state == MODEL_LOADING || self.state == MODEL_ACTIVE {
            return false;
        }
        let (tx, rx) = channel();
        let path = path.to_path_buf();
        let spawned = std::thread::Builder::new()
            .name("qj-model-load".to_owned())
            .spawn(move || {
                let started = std::time::Instant::now();
                let loaded = CharScorer::load(&path).and_then(|scorer| {
                    if p2c {
                        let scorer = P2cScorer::new(scorer)
                            .ok_or(NeuralError::Corrupt("含章·通变模型缺少 <sep> 分隔符"))?;
                        scorer.0.score_p2c("ni", &["你"])?;
                        Ok(Box::new(scorer) as Box<dyn SentenceScorer>)
                    } else {
                        scorer.score("", &["的"])?;
                        Ok(Box::new(scorer) as Box<dyn SentenceScorer>)
                    }
                });
                if loaded.is_ok() {
                    tracing::info!(
                        path = %path.display(),
                        p2c,
                        elapsed_ms = started.elapsed().as_millis(),
                        "本地模型已加载并预热"
                    );
                } else {
                    tracing::warn!(path = %path.display(), "本地模型加载失败");
                }
                // 接收端可能已随卸载丢掉，晚到的结果就地丢弃
                let _ = tx.send(loaded);
            });
        match spawned {
            Ok(_) => {
                self.loader = Some(rx);
                self.state = MODEL_LOADING;
                true
            }
            Err(error) => {
                tracing::warn!(%error, "起不了模型加载线程");
                self.state = MODEL_FAILED;
                false
            }
        }
    }

    /// 加载线程有结果就取：成功转在用并交出 scorer，失败记状态。没在加载或还没出结果返回 `None`。
    pub fn poll(&mut self) -> Option<Box<dyn SentenceScorer>> {
        if let Some(rx) = &self.loader {
            match rx.try_recv() {
                Ok(Ok(scorer)) => {
                    self.loader = None;
                    self.state = MODEL_ACTIVE;
                    return Some(scorer);
                }
                Ok(Err(_)) => {
                    self.loader = None;
                    self.state = MODEL_FAILED;
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    self.loader = None;
                }
            }
        }
        None
    }

    /// 卸载：状态回未加载。还在加载时晚到的结果由信道的 drop 丢弃。
    pub fn unload(&mut self) {
        self.loader = None;
        self.state = MODEL_IDLE;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bad_path_fails_and_reloads() {
        let mut state = ModelState::default();
        assert_eq!(state.state(), MODEL_IDLE);
        // 没有起线程失败的兜底路径可单测：坏文件走真加载，等它出结果
        assert!(state.load(Path::new("/nonexistent/model.qjm"), true));
        assert_eq!(state.state(), MODEL_LOADING);
        assert!(state.poll().is_none());
        let mut waited = 0;
        while state.state() == MODEL_LOADING {
            std::thread::sleep(std::time::Duration::from_millis(20));
            state.poll();
            waited += 20;
            assert!(waited < 10_000, "加载线程没有出结果");
        }
        assert_eq!(state.state(), MODEL_FAILED);
        // 失败后可以再来
        assert!(state.load(Path::new("/nonexistent/model.qjm"), true));
        state.unload();
        assert_eq!(state.state(), MODEL_IDLE);
    }
}
