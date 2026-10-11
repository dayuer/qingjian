//! 后台打分线程（B4 起兼做只动前缀缓存的预填 / 清空任务）。

use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::JoinHandle;
use std::time::Instant;

use crate::sentence::SentenceScorer;

use super::{GENERATE_BEAM, GENERATE_MAX_CHARS, NotifySlot};

/// 一次后台任务：两个条件与一批要打分的文本，外加可选的「直接按这段按键生成整句」。
/// 两件事合成一条任务是因为它们都在用户停顿后一起发出，而排队的任务只算最新一条。
struct Job {
    /// 提交序号（单调递增）：壳据此知道自己最新的那条任务回来了没有。
    id: u64,

    context: String,
    keys: String,
    texts: Vec<String>,

    /// 要生成整句的那段按键（整段作用域）；`None` 就只打分。
    generate: Option<String>,

    /// 不打分、只动模型里的前缀缓存（预填上文或清空）；这类任务不回结果。
    warmup: Option<Warmup>,

    /// 送出的时刻（算排队耗时）。
    submitted: Instant,
}

/// 只动模型前缀缓存的任务。
#[derive(Clone, Copy, PartialEq, Eq)]
enum Warmup {
    /// 把任务里的 `context` 算进缓存（[`SentenceScorer::prefill`]）。
    Prefill,

    /// 清掉缓存（[`SentenceScorer::forget`]）。
    Forget,
}

/// 后台算好的结果，与打分任务一一对应。
pub(crate) struct Scored {
    /// 对应任务的提交序号。
    pub id: u64,

    pub context: String,
    pub keys: String,
    pub texts: Vec<String>,
    pub scores: Vec<f64>,

    /// 生成用的按键与生成出来的整句，对应任务里的 `generate`。
    pub generated: Option<(String, Vec<String>)>,

    /// 在队列里等的毫秒数。
    pub queue_ms: f64,

    /// 前向打分的毫秒数。
    pub forward_ms: f64,

    /// 这次打分复用的前缀 token 数与前缀总长（[`SentenceScorer::prefix_reuse`]）。
    pub reuse: (usize, usize),

    /// 结果放进信道的时刻（主线程取走时算回主线程的耗时）。
    pub done: Instant,
}

/// 后台打分线程：模型前向要几十毫秒，不能放在按键回调里。
/// 任务排队时只算最新的一条（旧的对应已经过去的输入状态）；线程随本结构一起结束。
pub(crate) struct RescoreWorker {
    jobs: Sender<Job>,
    results: Receiver<Scored>,
    handle: Option<JoinHandle<()>>,
    next_id: AtomicU64,

    /// 打分器的分数依不依赖按键（[`SentenceScorer::uses_keys`]）：打分器进了线程就问不到了，起线程前问好。
    #[allow(dead_code)]
    uses_keys: bool,

    /// 最近一次送去预填的前文：同一段不重复送。只是省事，模型那边按 token 比较，送重了也不会算错。
    prefilled: RefCell<Option<String>>,
}

impl RescoreWorker {
    /// `notify` 里有回调时，每算完一条（结果已放进信道）就在后台线程上调一次，壳据此立刻来取，不用等下一拍轮询。
    pub fn spawn(scorer: Box<dyn SentenceScorer>, notify: NotifySlot) -> Self {
        let uses_keys = scorer.uses_keys();
        let (jobs, job_rx) = channel::<Job>();
        let (result_tx, results) = channel::<Scored>();
        let handle = std::thread::Builder::new()
            .name("qingjian-rescore".to_owned())
            .spawn(move || {
                while let Ok(first) = job_rx.recv() {
                    for job in latest_jobs(first, &job_rx) {
                        match job.warmup {
                            Some(Warmup::Prefill) => scorer.prefill(&job.context),
                            Some(Warmup::Forget) => scorer.forget(),
                            None => {
                                if result_tx.send(score_job(&*scorer, job)).is_err() {
                                    return;
                                }
                                if let Some(notify) =
                                    notify.lock().unwrap_or_else(|p| p.into_inner()).as_ref()
                                {
                                    notify();
                                }
                            }
                        }
                    }
                }
            })
            .ok();
        if handle.is_none() {
            tracing::warn!("起不了神经重打分线程，本次不用模型");
        }
        Self {
            jobs,
            results,
            handle,
            next_id: AtomicU64::new(1),
            uses_keys,
            prefilled: RefCell::new(None),
        }
    }

    pub fn is_alive(&self) -> bool {
        self.handle.is_some()
    }

    #[allow(dead_code)]
    pub fn uses_keys(&self) -> bool {
        self.uses_keys
    }

    pub fn submit(
        &self,
        context: String,
        keys: String,
        texts: Vec<String>,
        generate: Option<String>,
    ) -> u64 {
        self.send(context, keys, texts, generate, None)
    }

    /// 上文变了：后台先把它算进模型的前缀缓存。同一段前文只送一次。
    pub fn prefill(&self, context: &str) {
        let mut prefilled = self.prefilled.borrow_mut();
        if prefilled.as_deref() == Some(context) {
            return;
        }
        *prefilled = Some(context.to_owned());
        drop(prefilled);
        self.send(
            context.to_owned(),
            String::new(),
            Vec::new(),
            None,
            Some(Warmup::Prefill),
        );
    }

    /// 清掉模型里的前缀缓存（私密输入、换学习范围）。
    pub fn forget(&self) {
        *self.prefilled.borrow_mut() = None;
        self.send(
            String::new(),
            String::new(),
            Vec::new(),
            None,
            Some(Warmup::Forget),
        );
    }

    fn send(
        &self,
        context: String,
        keys: String,
        texts: Vec<String>,
        generate: Option<String>,
        warmup: Option<Warmup>,
    ) -> u64 {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        if self
            .jobs
            .send(Job {
                id,
                context,
                keys,
                texts,
                generate,
                warmup,
                submitted: Instant::now(),
            })
            .is_err()
        {
            tracing::warn!("神经重打分线程已退出");
        }
        id
    }

    /// 取一条打好的分；没有就 `None`。
    pub fn poll(&self) -> Option<Scored> {
        self.results.try_recv().ok()
    }
}

impl Drop for RescoreWorker {
    fn drop(&mut self) {
        // 关掉任务通道线程就会退出；不等它（模型可能正算到一半）
        let _ = self.handle.take();
    }
}

/// 把排着的任务收干净：打分只留最新一条，预填 / 清空也只留最新一条，两类之间按提交顺序做。
/// 不能让预填顶掉打分：被顶掉的打分永远不回结果，引擎会一直以为它在飞。
fn latest_jobs(first: Job, queue: &Receiver<Job>) -> Vec<Job> {
    let mut score = None;
    let mut warmup = None;
    for job in std::iter::once(first).chain(std::iter::from_fn(|| queue.try_recv().ok())) {
        if job.warmup.is_some() {
            warmup = Some(job);
        } else {
            score = Some(job);
        }
    }
    let mut batch: Vec<Job> = warmup.into_iter().chain(score).collect();
    batch.sort_by_key(|job| job.id);
    batch
}

/// 打一条分（连同可选的整句生成）。
fn score_job(scorer: &dyn SentenceScorer, job: Job) -> Scored {
    let texts: Vec<&str> = job.texts.iter().map(String::as_str).collect();
    let started = Instant::now();
    let queue_ms = started.duration_since(job.submitted).as_secs_f64() * 1000.0;
    let scores = scorer.score(&job.context, &job.keys, &texts);
    let forward_ms = started.elapsed().as_secs_f64() * 1000.0;
    let reuse = scorer.prefix_reuse();
    tracing::debug!(
        texts = texts.len(),
        context_chars = job.context.chars().count(),
        keys = job.keys.len(),
        ms = started.elapsed().as_millis(),
        reused = reuse.0,
        prefix = reuse.1,
        "神经重打分完成"
    );
    let generated = job.generate.map(|keys| {
        let started = Instant::now();
        let texts = scorer.generate(&keys, GENERATE_BEAM, GENERATE_MAX_CHARS);
        tracing::debug!(
            keys = keys.len(),
            got = texts.len(),
            ms = started.elapsed().as_millis(),
            "整句生成完成"
        );
        (keys, texts)
    });
    Scored {
        id: job.id,
        context: job.context,
        keys: job.keys,
        texts: job.texts,
        scores,
        generated,
        queue_ms,
        forward_ms,
        reuse,
        done: Instant::now(),
    }
}
