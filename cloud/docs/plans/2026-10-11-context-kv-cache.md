# B4 上文 / 拼音前缀的模型状态跨按键缓存 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** iOS 键盘上含章·通变每次重排不再整段重算拼音前缀，上文变了（上屏、移光标）由后台先算好，知微的词级分数表不再每键被清。

**Architecture:** qingjian-neural 里 `CharScorer` 的单格 K / V 缓存换成 `ContextCache`：同序列直接用、有公共前缀就截到公共前缀（`PrefixCache::truncate`）再只算新增 token（`CharLm::extend_cache`），超 4 MB 用完就丢。Core 的 `SentenceScorer` 加四个有缺省实现的方法（`prefill` / `forget` / `uses_keys` / `prefix_reuse`）；`RescoreWorker` 多两种「只动缓存」的任务（预填、清空），`set_rescoring_context` 前文变了送预填，进出私密与换学习范围时清空；知微的分数表不再拿按键当条件。壳不改结构，桥只多一列 `reused=` 日志和一行换人清空。

**Tech Stack:** Rust（candle 0.11）、qingjian-core / qingjian-neural / cloud 的 qingjian-cloud-bridge；iOS Swift 只改一行注释。

规格：[docs/notes/ref-ime-impl/b4-b6-context-cache-and-tiers.md](../../../docs/notes/ref-ime-impl/b4-b6-context-cache-and-tiers.md) 的 B4 节（§3.2 / §4）。**只做 B4**；B6 设备分档与「Core 的 `BEAM_WIDTH` / `RESCORE_PATHS` 可设」都不做。

---

## 前置 / 约定（仓库规矩，照做）

- 执行时开独立工作树，所有改动在那里做，主检出不动；PR 合了就删工作树：
  ```bash
  git -C /Users/liyuqing/sproot/qingjian fetch origin
  git -C /Users/liyuqing/sproot/qingjian worktree add ../qingjian-context-cache -b perf/context-kv-cache origin/sujian
  ```
  下文 `$W` 指 `/Users/liyuqing/sproot/qingjian-context-cache`，`$MAIN` 指 `/Users/liyuqing/sproot/qingjian`。
- PR 只开到自己的仓库：`gh pr create --repo dayuer/qingjian --base sujian`。**永远不往上游 qingjian-team/qingjian 开 PR、推送任何东西。** `gh` 命令一律带 `--repo dayuer/qingjian`。
- 提交信息：Conventional Commits，`<类型>(<范围>): <中文说明>`，范围用 crate / 壳名（core、neural、cloud、docs…），正文写为什么。**不要加 Co-Authored-By 或任何署名尾行**（`.githooks/commit-msg` 会拦格式）。
- 代码组织：一个类型一个文件；子模块用 `foo/mod.rs` 目录；同前缀兄弟文件收进目录；导入写精确路径不用 glob（`#[cfg(test)] mod tests` 里的 `use super::*` 除外；存量 glob 不专门清扫）；单文件 ≤ 800 行，测试超 200 行搬 `tests.rs`；新文件要 `//!` 头；注释中文、只写读代码看不出的约束；不写装饰性分隔注释。详见 `docs/contributing.md` 与 `CLAUDE.md`。
- 提交前：`cargo fmt --all`、`cargo clippy --all-targets -- -D warnings`（改了 `cloud/` 就在 `cloud/` 下也跑一遍）、相关 crate 的 `cargo test -p <crate>`。钩子启用一次：`git -C $W config core.hooksPath .githooks`。
- 排序 / 整句 / 纠错改动要跑回放与评测：`tools/release/gate-pass.sh` 与 crate-notes 里的 `--replay` / `--eval-text`。本计划的判据是**改前 / 改后输出逐字相同**（除「查询平均 / 最慢」耗时行），见 Task 9。
- 用户能感知的行为变了要同一提交改 `docs/user/`。本计划用户感知不到（只是快），不改 `docs/user/`；改了 crate 实现要点，同步 `docs/notes/crate-notes.md`（Task 10）。
- 改了上游已有文件（`crates/`、`apps/` 下非 `cloud/` 的文件）要在 `cloud/docs/fork-patch.md` 加一节「分叉补丁」（文件 | 改动 | 说明），原则是新代码放新文件、对上游文件只加挂钩行（Task 10）。
- 最后一个 Task 是推分支、开 PR，PR 描述写「怎么验证的」。**不合并**（只按用户明确指令合并）。

## 与规格的差异（以 origin/sujian 的真实代码为准）

计划里的代码在 `origin/sujian`（dd1ce657）的临时工作树里整套编过、跑过：neural 12 个测试、core 368 个、cloud bridge 全绿，clippy / fmt 干净。与规格 §3.2 / §4 的出入：

1. **`ContextCache` 的接口**：规格是 `empty()` + `sync() -> usize` + `cache()` + `bytes(cfg)`。实际写成 `new(max_bytes)` + `sync() -> PrefixCache`（交出句柄克隆，调用方马上放锁）+ `settle()`（用完超上限就丢）+ `last_reuse()`；字节数由新加的 `PrefixCache::bytes()` 按真实张量算，不按配置估。`MIN_REUSE_TOKENS` 不设：头一个 token 永远是 `<eos>`，公共前缀为 0 只在「被截空 / 构造」时出现，那时整段算。
2. **增量推进用新加的 `CharLm::extend_cache`**，不用规格说的 `step`：`step` 每次要把新段过输出层（8180 维 log-softmax），预填用不上。
3. **`PrefixCache` 加 `#[derive(Clone)]`**（张量句柄克隆，不拷数据）。
4. **清空用单独的 `forget()`，不用 `prefill("")`**：通变的缓存装的是拼音（私密框里敲的按键），通变的 `prefill` 又该是空操作（不看前文），两件事分不开就只能二选一。另加 `prefix_reuse()` 给统计用（规格 §4 第 6 步要的 `reused=`）。
5. **进出私密都清**：规格只写进私密时清；出私密时也清，私密框里敲的拼音前缀才不会带到普通框。
6. **「切换学习范围」在 Core 里没有入口**（Core 只有 `Learner::scope_changed`，换人发生在 cloud 桥的 `Session::switch_layers`）。做法：Core 加公开的 `Engine::forget_neural_context()`，桥在 `switch_layers` 里调一行。
7. **后台线程的「只算最新一条」要改**：原来新任务直接顶掉旧任务，预填任务排在打分任务后面会把打分顶掉，那条打分永远不回、`rescoring_in_flight` 一直为真、壳不再请求重排。改成「打分只留最新一条、预填 / 清空只留最新一条、两类按提交顺序做」（`latest_jobs`），有专门的测试。
8. **预填去重放在 `RescoreWorker`**（记最近送过的那段前文），不在 `set_rescoring_context` 里比新旧：桥每键都调 `set_rescoring_context`，后台模型那边本来就按 token 比较，送重了也不会算错，去重只是省一次线程唤醒。
9. **固定左截的宽度**：规格只说「`limit - MAX_TAIL`」。取 `TAIL_RESERVE = 32`（上下文 128 → 前文最多留 96 个 token）。缺省前文 64 字时与原来的截法逐位相同；只有候选超过 31 字时才额外截（与原来一样）。
10. **`P2c::convert`（整句生成）不改**：iOS 键盘关了生成（`set_sentence_generation(false)`），Mac 的生成与打分同一条后台任务、键相同，可以以后再接同一格缓存；不在本计划。
11. **测试位置**：`scorer.rs` 的测试段已近 190 行，新测试放进新模块 `context_cache/tests.rs` 与 `model/tests.rs`；Core 的放 `engine/rescoring/prefill/tests.rs`。

## File Structure

| 文件 | 新建 / 修改 | 职责 |
|---|---|---|
| `crates/qingjian-neural/src/model/mod.rs` | 改（上游） | `PrefixCache` 加 `Clone`、`truncate`、`bytes`；`CharLm::extend_cache`；`mod tests;` |
| `crates/qingjian-neural/src/model/tests.rs` | 新 | 截断 + 增量 ≈ 整段 |
| `crates/qingjian-neural/src/context_cache/mod.rs` | 新 | `ContextCache` 与 `CONTEXT_CACHE_MAX_BYTES` |
| `crates/qingjian-neural/src/context_cache/tests.rs` | 新 | 复用规则、两档容差、通变逐键、预填 |
| `crates/qingjian-neural/src/lib.rs` | 改（上游，1 行） | `mod context_cache;` |
| `crates/qingjian-neural/src/scorer.rs` | 改（上游） | 缓存换 `ContextCache`；`context_head` 固定左截；`sync_prefix` / `prefill` / `forget` / `prefix_reuse` |
| `crates/qingjian-neural/src/continuation.rs` | 改（分叉自有，1 行） | 续写的前文走同一格缓存 |
| `crates/qingjian-neural/src/core_scorer.rs` | 改（上游） | 两个实现接上新 trait 方法 |
| `crates/qingjian-neural/examples/context_cache_bench.rs` | 新 | 连续 10 键 / 上屏后第一次打分的延迟对比 |
| `crates/qingjian-core/src/sentence/scorer.rs` | 改（上游） | trait 加 4 个缺省方法 |
| `crates/qingjian-core/src/engine/rescoring/worker.rs` | 改（上游） | 预填 / 清空任务、`latest_jobs`、`uses_keys`、复用数进结果 |
| `crates/qingjian-core/src/engine/rescoring/prefill/mod.rs` | 新 | `Engine::prefill_rescoring_context` / `forget_neural_context` |
| `crates/qingjian-core/src/engine/rescoring/prefill/tests.rs` | 新 | 预填去重、私密清空、不顶掉打分、统计、知微分数表跨键 |
| `crates/qingjian-core/src/engine/rescoring/mod.rs` | 改（上游，挂钩行） | `mod prefill;`、`set_rescoring_context` 调预填、统计两字段 |
| `crates/qingjian-core/src/engine/rescoring/stats.rs` | 改（分叉自有） | `reused_tokens` / `prefix_tokens` |
| `crates/qingjian-core/src/engine/privacy.rs` | 改（上游，挂钩行） | 进出私密调 `forget_neural_context` |
| `crates/qingjian-core/src/engine/rescoring/word_rescore/mod.rs` | 改（分叉自有） | 知微不看按键时按键不进分数表条件 |
| `cloud/crates/qingjian-cloud-bridge/src/session/rescore.rs` | 改 | 统计串加 `reused=复用/前缀` |
| `cloud/crates/qingjian-cloud-bridge/src/session/memory/mod.rs` | 改 | 换人时 `forget_neural_context` |
| `cloud/crates/qingjian-cloud-bridge/src/lib.rs`、`cloud/ios/Keyboard/Sources/Engine.swift` | 改注释 | 统计串格式说明 |
| `docs/notes/crate-notes.md`、`cloud/docs/fork-patch.md` | 改 | 实现要点、分叉补丁 |

---

### Task 0: 工作树、数据与改前基线

**Files:** 无代码改动（只建工作树与 `/tmp` 下的基线文件）

- [ ] **Step 1: 建工作树、启用钩子**

```bash
git -C /Users/liyuqing/sproot/qingjian fetch origin
git -C /Users/liyuqing/sproot/qingjian worktree add ../qingjian-context-cache -b perf/context-kv-cache origin/sujian
W=/Users/liyuqing/sproot/qingjian-context-cache; MAIN=/Users/liyuqing/sproot/qingjian
git -C $W config core.hooksPath .githooks
```

- [ ] **Step 2: 补 gitignore 的数据**

模型测试按 `crates/qingjian-neural/../../data/models/` 找模型，**没有就静默跳过**——不补数据，Task 1–3 的测试全是假绿。`gate-pass.sh` 会往 `data/generated/` 写 `GATE_PASSED`，所以 `generated` 用 APFS 克隆拷一份，不要软链到主检出。

```bash
mkdir -p $W/data
ln -s $MAIN/data/models $W/data/models
ln -s $MAIN/data/eval $W/data/eval
cp -Rc $MAIN/data/generated $W/data/generated
cp -Rc $MAIN/data/generated.shipped $W/data/generated.shipped
ls $W/data/models/hanzhang-zhiwei $W/data/models/hanzhang-tongbian $W/data/models/hanzhang-tongbian-q8
```
Expected：三个目录各有一个 `.qjm`（知微 `hanzhang-zhiwei-small.qjm`、通变 `hanzhang-tongbian-small.qjm`、8 位 `tongbian-q8.qjm`）。

- [ ] **Step 3: 改前的 CLI 留一份、跑改前的评测与回放**

```bash
cd $W
cargo build --quiet --release -p qingjian-cli && cp target/release/qingjian-cli /tmp/qj-cli-base
M=data/models/hanzhang-tongbian/hanzhang-tongbian-small.qjm
/tmp/qj-cli-base --config tools/eval/offline.toml --neural $M --eval-text data/eval/sentences.tsv \
  --extra-dict data/generated/dicts/idioms.qj 2>/dev/null > /tmp/b4-eval-base.txt
/tmp/qj-cli-base --config tools/eval/offline.toml --neural $M --replay data/eval/input-log-2026-10-04.jsonl \
  --clean2 --extra-dict data/generated/dicts/idioms.qj --misses 0 2>/dev/null > /tmp/b4-replay-base.txt
tools/release/gate-pass.sh 2>&1 | tee /tmp/b4-gate-base.txt | tail -15
```
Expected：两份输出非空；`gate-pass.sh` 末尾打出「配置 / 指标 / 基线 / 现在 / 差」表。**改前若已经没过门槛**（本机 `data/generated` 与门槛基线 sujian-data-v1 不是同一份时会这样），照记下来，不是本计划造成的；本计划的判据是 Task 9 里「改前 / 改后逐字相同」。

---

### Task 1: `PrefixCache` 可截断、`CharLm` 可增量接续

**Files:**
- Modify: `crates/qingjian-neural/src/model/mod.rs`（`mod weight;` 后、`PrefixCache` 定义与 impl、`CharLm::prefix_cache` 之后）
- Create: `crates/qingjian-neural/src/model/tests.rs`

- [ ] **Step 1: 写失败的测试**

在 `model/mod.rs` 顶部 `mod weight;` 之后加：

```rust
#[cfg(test)]
mod tests;
```

新建 `crates/qingjian-neural/src/model/tests.rs`：

```rust
//! `PrefixCache::truncate` 与 `CharLm::extend_cache`：截到前 k 个位置再接着推进，与整段算的前缀一致。
//! 知微模型不在 `data/models/` 下就跳过。

use std::path::Path;

use candle_core::Tensor;

use super::{CharLm, PrefixCache};
use crate::CharScorer;
use crate::vocab::EOS;

fn zhiwei() -> Option<CharScorer> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/models/hanzhang-zhiwei");
    dir.exists().then(|| CharScorer::load(&dir).unwrap())
}

/// 接在缓存后面喂一个 token，取下一个 token 的 log 概率分布。
fn next_log_probs(model: &CharLm, cache: &PrefixCache, token: u32) -> Vec<f32> {
    let idx = Tensor::from_vec(vec![token], (1, 1), model.device()).unwrap();
    model
        .log_probs_after(cache, &idx)
        .unwrap()
        .flatten_all()
        .unwrap()
        .to_vec1::<f32>()
        .unwrap()
}

#[test]
fn truncate_then_extend_matches_full_prefix() {
    let Some(scorer) = zhiwei() else {
        eprintln!("没有知微模型，跳过");
        return;
    };
    let model = scorer.model();
    // f32 CPU 路径 1e-4；Metal 上是 f16，放宽到 8 位同口径
    let tolerance = if model.device().is_metal() {
        0.08
    } else {
        1e-4
    };
    let all: Vec<u32> = std::iter::once(EOS)
        .chain(scorer.vocab().encode("我今天想去上海看看"))
        .collect();
    let full = model.prefix_cache(&all).unwrap();
    let expected = next_log_probs(model, &full, all[1]);
    for keep in [1, 3, all.len() - 1] {
        let part = full.truncate(keep).unwrap();
        assert_eq!(part.len(), keep);
        let grown = model.extend_cache(&part, &all[keep..]).unwrap();
        assert_eq!(grown.len(), all.len());
        let got = next_log_probs(model, &grown, all[1]);
        let diff = expected
            .iter()
            .zip(&got)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);
        assert!(diff < tolerance, "keep={keep}: {diff}");
    }
    assert!(full.truncate(all.len() + 1).is_err());
    assert!(full.truncate(0).unwrap().is_empty());
    assert!(full.bytes() > 0);
    assert_eq!(full.truncate(0).unwrap().bytes(), 0);
}
```

- [ ] **Step 2: 跑测试，确认编译失败**

Run: `cd $W && cargo test -p qingjian-neural model::tests`
Expected: 编译错误 `no method named truncate found for struct PrefixCache` / `no method named extend_cache`。

- [ ] **Step 3: 实现**

`PrefixCache` 的文档注释末尾加一行并派生 `Clone`：

```rust
/// 一段前文在每层的 K / V（形状 `[1, h, p, d]`）。前文在一次组句里不变，算一次存下来，
/// 候选接在后面时只算候选自己那几个 token（见 [`CharLm::log_probs_after`]）。
/// 克隆只复制张量句柄，不复制数据。
#[derive(Clone)]
pub struct PrefixCache {
```

`impl PrefixCache` 里 `select` 之后加：

```rust
    /// 只留前 `len` 个位置（`len` 不超过现有长度）：前文被改短或改了中间时，公共前缀那段不用重算。
    /// 每层 `narrow` 出视图，不拷数据；下次接着推进时 `attention` 会拼出新张量。
    pub fn truncate(&self, len: usize) -> Result<Self> {
        if len > self.len {
            candle_core::bail!("truncate to {len} beyond cached {}", self.len);
        }
        if len == 0 {
            return Ok(Self {
                keys: Vec::new(),
                values: Vec::new(),
                len: 0,
            });
        }
        let cut = |t: &Tensor| t.narrow(2, 0, len);
        Ok(Self {
            keys: self.keys.iter().map(cut).collect::<Result<_>>()?,
            values: self.values.iter().map(cut).collect::<Result<_>>()?,
            len,
        })
    }

    /// K / V 占的字节数（按可见元素算）。
    pub fn bytes(&self) -> usize {
        self.keys
            .iter()
            .chain(&self.values)
            .map(|t| t.elem_count() * t.dtype().size_in_bytes())
            .sum()
    }
```

`impl CharLm` 里 `prefix_cache` 之后、`step` 之前加：

```rust
    /// 接着缓存推进一段 token（batch 为 1），只要长出来的缓存、不算输出层。前文追加时增量预填用；
    /// 对空缓存调用等同 [`Self::prefix_cache`]。
    pub fn extend_cache(&self, cache: &PrefixCache, ids: &[u32]) -> Result<PrefixCache> {
        if ids.is_empty() {
            return Ok(cache.clone());
        }
        let idx = Tensor::from_vec(ids.to_vec(), (1, ids.len()), &self.device)?;
        let (_, grown) = self.hidden(&idx, Some(cache), true)?;
        Ok(grown.expect("record was requested"))
    }
```

（`hidden` 对空缓存走 `past.filter(|c| !c.is_empty())` 那条分支，所以截到 0 再接续就是整段算；`attention` 对 `narrow` 出来的视图先 `broadcast_as(..).contiguous()`，不用额外拷。）

- [ ] **Step 4: 跑测试，确认通过**

Run: `cd $W && cargo test -p qingjian-neural model::tests -- --nocapture`
Expected: `test model::tests::truncate_then_extend_matches_full_prefix ... ok`（不是「没有知微模型，跳过」——看到跳过说明 Task 0 Step 2 没做）。原型实测 CPU f32 最大差 2.5e-5。

- [ ] **Step 5: 提交**

```bash
cd $W && cargo fmt --all && cargo clippy -p qingjian-neural --all-targets -- -D warnings
git add crates/qingjian-neural/src/model/
git commit -m "perf(neural): 前缀 K/V 可截断、可增量接续" -m "PrefixCache::truncate 按位置 narrow 出视图，CharLm::extend_cache 接着缓存只算新增 token、不过输出层。
给 B4 的跨按键前缀复用打底：拼音退格是截断、打字是追加，上文刚上屏是追加。"
```

---

### Task 2: `ContextCache`（同序列直接用 / 公共前缀截断后续算）

**Files:**
- Create: `crates/qingjian-neural/src/context_cache/mod.rs`
- Create: `crates/qingjian-neural/src/context_cache/tests.rs`
- Modify: `crates/qingjian-neural/src/lib.rs`（`mod config;` 之后）

本 Task 结束时 `ContextCache` 还只有测试在用，`clippy -D warnings` 会报 dead code，所以**不单独提交**，与 Task 3 一起提交。

- [ ] **Step 1: 写失败的测试**

`lib.rs` 在 `mod config;` 之后加 `mod context_cache;`（按字母序在 `mod continuation;` 前）。

新建 `crates/qingjian-neural/src/context_cache/tests.rs`：

```rust
//! 前缀缓存的复用规则与数值：命中逐位相等；增量与整段算的差在容差内（f32 1e-4，8 位与 Metal f16 路径 0.08），名次一致。
//! 模型不在 `data/models/` 下就跳过（与 scorer.rs 的测试同口径）。

use std::path::{Path, PathBuf};

use candle_core::Tensor;

use super::ContextCache;
use crate::model::PrefixCache;
use crate::vocab::EOS;
use crate::{CharLm, CharScorer};

fn model_path(dir: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/models")
        .join(dir);
    path.exists().then_some(path)
}

fn zhiwei() -> Option<CharScorer> {
    model_path("hanzhang-zhiwei").map(|p| CharScorer::load(&p).unwrap())
}

/// f32 CPU 路径的容差；Metal 上是 f16，放宽到与 8 位同口径。
fn tolerance(model: &CharLm) -> f32 {
    if model.device().is_metal() {
        0.08
    } else {
        1e-4
    }
}

/// 接在缓存后面喂一个 token，取下一个 token 的 log 概率分布。
fn next_log_probs(model: &CharLm, cache: &PrefixCache, token: u32) -> Vec<f32> {
    let idx = Tensor::from_vec(vec![token], (1, 1), model.device()).unwrap();
    model
        .log_probs_after(cache, &idx)
        .unwrap()
        .flatten_all()
        .unwrap()
        .to_vec1::<f32>()
        .unwrap()
}

fn max_diff(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f32::max)
}

fn ids(scorer: &CharScorer, text: &str) -> Vec<u32> {
    std::iter::once(EOS)
        .chain(scorer.vocab().encode(text))
        .collect()
}

#[test]
fn sync_reuses_the_common_prefix() {
    let Some(scorer) = zhiwei() else {
        eprintln!("没有知微模型，跳过");
        return;
    };
    let model = scorer.model();
    let probe = |cache: &PrefixCache| next_log_probs(model, cache, EOS);
    let reference = |seq: &[u32]| probe(&model.prefix_cache(seq).unwrap());
    let mut cache = ContextCache::new(usize::MAX);
    let a = ids(&scorer, "我今");
    let b = ids(&scorer, "我今天");
    let c = ids(&scorer, "我今天想");
    let d = ids(&scorer, "我们");

    // 命中：同一序列两次，逐位相等
    let first = probe(&cache.sync(model, &a).unwrap());
    assert_eq!(cache.last_reuse(), (0, a.len()));
    let again = probe(&cache.sync(model, &a).unwrap());
    assert_eq!(cache.last_reuse(), (a.len(), a.len()));
    assert_eq!(first, again);

    // 追加：只算新增的那个字
    let appended = probe(&cache.sync(model, &b).unwrap());
    assert_eq!(cache.last_reuse(), (a.len(), b.len()));
    assert!(max_diff(&appended, &reference(&b)) < tolerance(model));

    // 退格：截断，零前向
    cache.sync(model, &c).unwrap();
    let shortened = probe(&cache.sync(model, &b).unwrap());
    assert_eq!(cache.last_reuse(), (b.len(), b.len()));
    assert!(max_diff(&shortened, &reference(&b)) < tolerance(model));

    // 分叉：留公共前缀 `<eos>我`
    let forked = probe(&cache.sync(model, &d).unwrap());
    assert_eq!(cache.last_reuse(), (2, d.len()));
    assert!(max_diff(&forked, &reference(&d)) < tolerance(model));

    // 没有公共前缀：整段重算
    let other: Vec<u32> = d[1..].to_vec();
    cache.sync(model, &other).unwrap();
    assert_eq!(cache.last_reuse(), (0, other.len()));

    // 空前缀也能算（候选很长时打分的前缀会被截空）
    assert!(cache.sync(model, &[]).unwrap().is_empty());
}

#[test]
fn over_budget_cache_is_not_kept() {
    let Some(scorer) = zhiwei() else {
        eprintln!("没有知微模型，跳过");
        return;
    };
    let model = scorer.model();
    let seq = ids(&scorer, "我今天");
    let mut cache = ContextCache::new(1);
    let first = next_log_probs(model, &cache.sync(model, &seq).unwrap(), EOS);
    cache.settle();
    assert!(cache.is_empty());
    let second = next_log_probs(model, &cache.sync(model, &seq).unwrap(), EOS);
    assert_eq!(cache.last_reuse(), (0, seq.len()), "超上限的每次现算");
    assert_eq!(first, second);
}
```

新建 `crates/qingjian-neural/src/context_cache/mod.rs`，先只放模块头与测试声明：

```rust
//! 一段前缀（光标前文或通变的拼音）的 K / V 跨调用复用：完全相同直接用，新序列与旧序列有公共前缀就截到公共前缀、
//! 只算多出来的 token，没有公共前缀才整段重算。对应 RefIME `DecoderOnlyNNModel::PrefillContext`，
//! 规则与取舍见 docs/notes/ref-ime-impl/b4-b6-context-cache-and-tiers.md 的 B4 节。

#[cfg(test)]
mod tests;
```

- [ ] **Step 2: 跑测试，确认编译失败**

Run: `cd $W && cargo test -p qingjian-neural context_cache`
Expected: 编译错误 `unresolved import super::ContextCache`。

- [ ] **Step 3: 实现**

把 `context_cache/mod.rs` 写全：

```rust
//! 一段前缀（光标前文或通变的拼音）的 K / V 跨调用复用：完全相同直接用，新序列与旧序列有公共前缀就截到公共前缀、
//! 只算多出来的 token，没有公共前缀才整段重算。对应 RefIME `DecoderOnlyNNModel::PrefillContext`，
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
```

- [ ] **Step 4: 跑测试，确认通过**

Run: `cd $W && cargo test -p qingjian-neural context_cache -- --nocapture`
Expected: `sync_reuses_the_common_prefix ... ok`、`over_budget_cache_is_not_kept ... ok`。
这两条覆盖规格单测清单里的「sync 命中（逐位相等）/ 追加 / 退格（截断零前向）/ 分叉 / 无公共前缀 / 超预算不留」。

（不提交，接 Task 3。）

---

### Task 3: `CharScorer` 接上 `ContextCache`（固定左截、预填、清空、复用数）

**Files:**
- Modify: `crates/qingjian-neural/src/scorer.rs`
- Modify: `crates/qingjian-neural/src/continuation.rs:35`
- Modify: `crates/qingjian-neural/src/context_cache/tests.rs`（追加）

- [ ] **Step 1: 写失败的测试**

在 `context_cache/tests.rs` 末尾追加：

```rust
/// 通变：有 8 位的先用 8 位（iOS 键盘装的就是它），否则用 fp16 那份。
fn tongbian() -> Option<CharScorer> {
    model_path("hanzhang-tongbian-q8")
        .or_else(|| model_path("hanzhang-tongbian"))
        .map(|p| CharScorer::load(&p).unwrap())
}

/// 通变逐键：每键只接新字母，分数与整段算的差在 0.08 内、名次一致；第 2 键起复用上一键的前缀。
#[test]
fn p2c_keystrokes_reuse_the_pinyin_prefix() {
    let Some(scorer) = tongbian() else {
        eprintln!("没有通变模型，跳过");
        return;
    };
    let texts = ["我们明天", "我们", "我门明天", "沃们"];
    let keys = "womenmingtian";
    let mut previous_head = 0;
    // 每键先按缓存算，再清空整段算一遍对拍；下一键接着整段算出来的前缀长，与真实打字一样
    for end in 1..=keys.len() {
        let typed = &keys[..end];
        let warm = scorer.score_p2c(typed, &texts).unwrap();
        let (reused, head) = scorer.prefix_reuse();
        if end > 1 {
            assert_eq!(reused, previous_head, "{typed}");
        }
        previous_head = head;
        scorer.forget();
        let cold = scorer.score_p2c(typed, &texts).unwrap();
        assert_eq!(scorer.prefix_reuse().0, 0);
        for (w, c) in warm.iter().zip(&cold) {
            assert!((w - c).abs() < 0.08, "{typed}: {warm:?} vs {cold:?}");
        }
        assert_eq!(rank(&warm), rank(&cold), "{typed}");
    }
    // 退格：截断
    scorer.score_p2c("womenming", &texts).unwrap();
    scorer.score_p2c("womenmin", &texts).unwrap();
    let (reused, head) = scorer.prefix_reuse();
    assert_eq!(reused, head);
}

fn rank(scores: &[f64]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..scores.len()).collect();
    order.sort_by(|&a, &b| scores[b].total_cmp(&scores[a]));
    order
}

/// 预填后打分直接命中，分数与不预填一致；追加上文（刚上屏）时只算新增的字；清空后不留。
#[test]
fn prefill_then_score_hits_the_cache() {
    let Some(scorer) = zhiwei() else {
        eprintln!("没有知微模型，跳过");
        return;
    };
    let texts = ["上海", "伤害", "吃饭"];
    let cold = scorer.score("我今天想去", &texts).unwrap();
    scorer.forget();
    assert!(!scorer.prefix_cached());
    scorer.prefill("我今天想").unwrap();
    scorer.prefill("我今天想去").unwrap();
    let (reused, head) = scorer.prefix_reuse();
    assert!(reused > 0 && reused < head, "追加只算新增：{reused}/{head}");
    let warm = scorer.score("我今天想去", &texts).unwrap();
    let (reused, head) = scorer.prefix_reuse();
    assert_eq!(reused, head, "打分时整段命中");
    for (w, c) in warm.iter().zip(&cold) {
        assert!(
            (w - c).abs() < f64::from(tolerance(scorer.model())) * 10.0,
            "{warm:?} vs {cold:?}"
        );
    }
    assert_eq!(rank(&warm), rank(&cold));
    scorer.forget();
    assert!(!scorer.prefix_cached());
}
```

- [ ] **Step 2: 跑测试，确认编译失败**

Run: `cd $W && cargo test -p qingjian-neural context_cache`
Expected: 编译错误 `no method named prefix_reuse` / `forget` / `prefill` / `prefix_cached`。

- [ ] **Step 3: 实现 `scorer.rs`**

文件开头加文件头，导入改成：

```rust
//! 加载好的字级模型：给「前文 + 候选」（知微）或「拼音 + 候选」（通变）打分；前缀的 K / V 跨调用留一格（[`ContextCache`]）。

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use candle_core::{DType, Device, Tensor};
use candle_nn::VarBuilder;
use qingjian_format::{Container, Kind, Metadata};

use crate::context_cache::{CONTEXT_CACHE_MAX_BYTES, ContextCache};
use crate::model::PrefixCache;
use crate::vocab::EOS;
use crate::{CharLm, ModelConfig, NeuralError, QuantizedWeights, Vocab, find_model, qjm};
```

`cache` 字段换类型，结构体后加常数：

```rust
    /// 最近一段前缀（知微是前文，通变是拼音）的 K / V：前缀不变只算候选，前缀长了或短了只算差的那段。
    cache: Mutex<ContextCache>,
}

/// 知微打分时前文最多留这么多 token（含开头的 `<eos>`），从左截：模型上下文减去候选的预留宽度 [`TAIL_RESERVE`]。
/// 截法只看前文、不看这批候选多长，预填（[`CharScorer::prefill`]）与打分才能算出同一个前缀。
const TAIL_RESERVE: usize = 32;
```

`with_model` 里 `cache: Mutex::new(None),` 改成 `cache: Mutex::new(ContextCache::new(CONTEXT_CACHE_MAX_BYTES)),`，并在 `with_model` 之后加：

```rust
    fn lock_cache(&self) -> MutexGuard<'_, ContextCache> {
        self.cache
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// 知微打分用的前缀：`<eos> + 前文` 去掉最后一个 token（它放进每行开头给候选第一个字用），
    /// 从左截到上下文减 [`TAIL_RESERVE`]。返回（前缀, 最后一个 token）。
    pub(crate) fn context_head(&self, context: &str) -> (Vec<u32>, u32) {
        let mut full: Vec<u32> = std::iter::once(EOS)
            .chain(self.vocab.encode(context))
            .collect();
        let last = full.pop().expect("has eos");
        let keep = self.model.config().context.saturating_sub(TAIL_RESERVE);
        if full.len() > keep {
            full.drain(..full.len() - keep);
        }
        (full, last)
    }

    /// 上文变了先在后台把它的 K / V 算好（RefIME `AutoInferTextBeforeCursor`），下一次 [`Self::score`] 直接命中。
    /// 新前文以旧前文开头（刚上屏）时只算新增的字。
    pub fn prefill(&self, context: &str) -> Result<(), NeuralError> {
        let (head, _) = self.context_head(context);
        self.sync_prefix(&head)?;
        Ok(())
    }

    /// 让前缀缓存对应 `ids` 并交出它的 K / V（句柄克隆，交出后就放锁）；超过字节上限的用完不留。
    pub(crate) fn sync_prefix(&self, ids: &[u32]) -> Result<PrefixCache, NeuralError> {
        let mut guard = self.lock_cache();
        let cache = guard.sync(&self.model, ids)?;
        guard.settle();
        Ok(cache)
    }

    /// 丢掉前缀缓存：私密输入、换学习范围时调，前文与拼音的状态不在内存里留着。
    pub fn forget(&self) {
        self.lock_cache().clear();
    }

    /// 最近一次打分的前缀复用：（复用的 token 数, 前缀总长）。
    pub fn prefix_reuse(&self) -> (usize, usize) {
        self.lock_cache().last_reuse()
    }

    /// 前缀缓存里有没有东西（测试看清空用）。
    #[cfg(test)]
    pub(crate) fn prefix_cached(&self) -> bool {
        !self.lock_cache().is_empty()
    }
```

`score_p2c` 里从 `let mut guard = self` 到 `drop(guard);` 那一段（原 161–171 行）换成：

```rust
        // 打字是追加、退格是截断：拼音前缀跨按键只算变了的那几个字母
        let cache = self.sync_prefix(&head)?;
        let picked = self.model.target_log_probs_after(&cache, &idx, &targets)?;
```

`score`：文档注释最后一行改成
`/// 前文先按 [`Self::context_head`] 固定截一次；前文加最长候选还超过模型上下文时再从左边截（与训练脚本 `score.py` 一致）。`；
删掉 `let full: Vec<u32> = …collect();` 三行；把

```rust
        // 缓存的是前文去掉最后一个 token 的部分，最后一个 token 放进每一行的开头，它的输出分布给候选第一个字用
        let (last, head) = full.split_last().expect("has eos");
        let head = &head[head.len().saturating_sub(limit - width)..];
```
换成
```rust
        let (head, last) = self.context_head(context);
        let head = &head[head.len().saturating_sub(limit - width)..];
```
`flat[row * width] = *last;` 改成 `flat[row * width] = last;`；末尾从 `let mut guard = self` 到 `drop(guard);` 换成：

```rust
        let cache = self.sync_prefix(head)?;
        let lp = self.model.log_probs_after(&cache, &idx)?;
```

- [ ] **Step 4: `continuation.rs` 走同一格缓存**

第 35 行 `let mut cache = self.model().prefix_cache(&ids[..split])?;` 换成：

```rust
        // 前文部分与 `score` 的前缀常常相同（同一段光标前文），走同一格缓存
        let mut cache = self.sync_prefix(&ids[..split])?;
```

- [ ] **Step 5: 跑整个 crate 的测试**

Run: `cd $W && cargo test -p qingjian-neural -- --nocapture 2>&1 | grep -E "^test |test result|跳过"`
Expected: 12 passed、1 ignored，没有任何「跳过」；新增的 `p2c_keystrokes_reuse_the_pinyin_prefix`、`prefill_then_score_hits_the_cache` 通过，原有 `p2c_cached_scores_match_full_forward`（P2C 与整段前向差 < 0.08）、`matches_the_python_scorer`、`continues_a_few_characters_and_stops_at_punctuation` 照旧通过。

- [ ] **Step 6: 提交（含 Task 2）**

```bash
cd $W && cargo fmt --all && cargo clippy -p qingjian-neural --all-targets -- -D warnings
git add crates/qingjian-neural/src/
git commit -m "perf(neural): 前缀 K/V 跨调用复用，通变每键只算新字母" -m "CharScorer 的单格缓存原来要求前缀完全相等，通变的前缀是拼音、每键多一个字母，每次都整段重算。
ContextCache 按公共前缀截断再增量接续：打字是追加、退格是截断，上文刚上屏只算新增的字；超 4MB 用完就丢。
知微的前文改成只看前文的固定左截（TAIL_RESERVE=32），预填与打分算出同一个前缀；缺省 64 字前文时与原截法逐位相同。
加 prefill / forget / prefix_reuse，给 Core 的后台预填、私密清空与真机日志用。"
```

---

### Task 4: `SentenceScorer` 的四个新方法与两个实现

**Files:**
- Modify: `crates/qingjian-core/src/sentence/scorer.rs`（trait 末尾）
- Modify: `crates/qingjian-neural/src/core_scorer.rs`

- [ ] **Step 1: trait 加缺省方法**

`sentence/scorer.rs` 里 `continue_text` 之后、trait 结尾 `}` 之前加：

```rust
    /// 上文变了，先在后台把上文的模型状态算好，下一次 [`Self::score`] 直接用（RefIME `AutoInferTextBeforeCursor`）。
    /// 不看前文的实现（P2C）什么都不做。
    fn prefill(&self, _context: &str) {}

    /// 丢掉模型里缓存的前文 / 按键状态：私密输入、换学习范围时调。
    fn forget(&self) {}

    /// 分数是否依赖用户按键。字级模型不依赖：按键变了分数表不必清。
    fn uses_keys(&self) -> bool {
        true
    }

    /// 最近一次打分的前缀复用：（复用的 token 数, 前缀总长），真机日志看命中率；没有缓存的实现返回 (0, 0)。
    fn prefix_reuse(&self) -> (usize, usize) {
        (0, 0)
    }
```

- [ ] **Step 2: neural 两个实现接上**

`core_scorer.rs` 文件开头加文件头：

```rust
//! 把 [`CharScorer`] 接到 Core 的 [`SentenceScorer`]：知微按前文打分，通变（[`P2cScorer`]）按按键打分与生成。

```

`impl SentenceScorer for CharScorer` 里 `score` 之后加：

```rust
    fn prefill(&self, context: &str) {
        if let Err(error) = CharScorer::prefill(self, context) {
            tracing::warn!(%error, "上文预填失败，打分时再算");
        }
    }

    fn forget(&self) {
        CharScorer::forget(self);
    }

    fn uses_keys(&self) -> bool {
        false
    }

    fn prefix_reuse(&self) -> (usize, usize) {
        CharScorer::prefix_reuse(self)
    }
```

`impl SentenceScorer for P2cScorer` 里 `score` 之后、`generate` 之前加（`prefill` 保持缺省空操作：通变不看前文；`uses_keys` 保持缺省 true）：

```rust
    fn forget(&self) {
        self.0.forget();
    }

    fn prefix_reuse(&self) -> (usize, usize) {
        self.0.prefix_reuse()
    }
```

- [ ] **Step 3: 编译与测试**

Run: `cd $W && cargo check --workspace --all-targets && cargo test -p qingjian-core -p qingjian-neural 2>&1 | grep "test result"`
Expected: 全部 ok（缺省实现不改任何现有行为；Windows / Mac 壳里别的 `SentenceScorer` 实现不用动）。

- [ ] **Step 4: 提交**

```bash
cd $W && cargo fmt --all && cargo clippy -p qingjian-core -p qingjian-neural --all-targets -- -D warnings
git add crates/qingjian-core/src/sentence/scorer.rs crates/qingjian-neural/src/core_scorer.rs
git commit -m "feat(core): SentenceScorer 加预填、清空、是否看按键、前缀复用数四个缺省方法" -m "Core 仍不知道模型细节：预填与清空是给后台线程的指令，uses_keys 让知微的分数表不必每键清，prefix_reuse 进真机日志。
缺省实现保持旧行为；知微实现前三个，通变只实现清空（它的缓存是拼音）与复用数。"
```

---

### Task 5: Core 的预填与清空（后台任务、私密、统计）

**Files:**
- Modify: `crates/qingjian-core/src/engine/rescoring/worker.rs`（整文件替换）
- Create: `crates/qingjian-core/src/engine/rescoring/prefill/mod.rs`
- Create: `crates/qingjian-core/src/engine/rescoring/prefill/tests.rs`
- Modify: `crates/qingjian-core/src/engine/rescoring/mod.rs`（`mod` 列表、`set_rescoring_context`、`poll_rescoring` 里的 `RescoreStats`）
- Modify: `crates/qingjian-core/src/engine/rescoring/stats.rs`
- Modify: `crates/qingjian-core/src/engine/privacy.rs`（`set_private_from`）

- [ ] **Step 1: 写失败的测试**

`rescoring/mod.rs` 的 `mod cache;` 后加 `mod prefill;`。新建 `rescoring/prefill/mod.rs`，先只放：

```rust
//! 上文的模型状态跨按键缓存（B4）在 Core 这一侧的两件事：上文变了把预填任务送去后台，
//! 私密输入 / 换学习范围时让模型丢掉缓存的前文与按键状态。模型那一侧见 qingjian-neural 的 `ContextCache`。

#[cfg(test)]
mod tests;
```

新建 `rescoring/prefill/tests.rs`：

```rust
//! B4 在 Core 一侧的行为：前文变了送预填（同一段只送一次）、私密进出清缓存、知微分数表跨按键保留、
//! 预填不顶掉排着的打分、复用数进统计。假打分器，不依赖模型文件。

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use qingjian_dictionary::Dictionary;

use crate::engine::Engine;
use crate::sentence::{Conversion, SentenceScorer, SentenceWord};

/// 记下自己被调了什么的假打分器。
#[derive(Clone, Default)]
struct Recorder {
    events: Arc<Mutex<Vec<String>>>,

    /// 分数不看按键（像知微）。
    keyless: bool,

    /// 预填与打分各睡这么久，用来让任务在队列里排起来。
    delay: Duration,
}

impl Recorder {
    fn events(&self) -> Vec<String> {
        self.events.lock().unwrap().clone()
    }

    fn push(&self, event: String) {
        self.events.lock().unwrap().push(event);
    }

    /// 等到某条事件出现（后台线程上发生的）。
    fn wait_for(&self, event: &str) {
        let started = Instant::now();
        while !self.events().iter().any(|e| e == event) {
            assert!(
                started.elapsed() < Duration::from_secs(5),
                "等不到 {event}：{:?}",
                self.events()
            );
            std::thread::sleep(Duration::from_millis(2));
        }
    }
}

impl SentenceScorer for Recorder {
    fn score(&self, _context: &str, _keys: &str, texts: &[&str]) -> Vec<f64> {
        std::thread::sleep(self.delay);
        self.push(format!("score:{}", texts.join(",")));
        texts
            .iter()
            .map(|t| if *t == "油箱" { -1.0 } else { -20.0 })
            .collect()
    }

    fn prefill(&self, context: &str) {
        std::thread::sleep(self.delay);
        self.push(format!("prefill:{context}"));
    }

    fn forget(&self) {
        self.push("forget".to_owned());
    }

    fn uses_keys(&self) -> bool {
        !self.keyless
    }

    fn prefix_reuse(&self) -> (usize, usize) {
        (3, 5)
    }
}

const WORDS: &str = "邮箱\tyou xiang\t9000\n油箱\tyou xiang\t3000\n邮\tyou\t900\n又\tyou\t800\n想\txiang\t900\n开发\tkai fa\t9000\n";

fn engine() -> Engine {
    Engine::new(Dictionary::parse(WORDS).unwrap())
}

fn path(text: &str, score: f64) -> Conversion {
    Conversion {
        text: text.to_owned(),
        syllables: Vec::new(),
        words: vec![SentenceWord {
            text: text.to_owned(),
            syllables: Vec::new(),
            placeholder: false,
        }],
        score,
        static_score: score,
        penalty: 0.0,
    }
}

#[test]
fn changed_context_is_prefilled_once() {
    let word = Recorder {
        keyless: true,
        ..Recorder::default()
    };
    let mut engine = engine().with_async_word_scorer(Box::new(word.clone()), Some(1.0));
    engine.set_rescoring_context(Some("我今天想".to_owned()));
    engine.set_rescoring_context(Some("我今天想".to_owned()));
    engine.set_rescoring_context(Some("我今天想去".to_owned()));
    word.wait_for("prefill:我今天想去");
    let prefills = word
        .events()
        .iter()
        .filter(|e| e.starts_with("prefill:"))
        .count();
    assert!(prefills <= 2, "同一段只送一次：{:?}", word.events());
}

#[test]
fn private_input_forgets_and_does_not_prefill() {
    let word = Recorder {
        keyless: true,
        ..Recorder::default()
    };
    let sentence = Recorder::default();
    let mut engine = engine()
        .with_async_sentence_scorer(Box::new(sentence.clone()), Some(0.5), None, None)
        .with_async_word_scorer(Box::new(word.clone()), Some(1.0));
    engine.set_private(true);
    word.wait_for("forget");
    sentence.wait_for("forget");
    engine.set_rescoring_context(Some("验证码 123456".to_owned()));
    engine.set_private(false);
    let started = Instant::now();
    while word.events().iter().filter(|e| *e == "forget").count() < 2 {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "{:?}",
            word.events()
        );
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(
        word.events().iter().all(|e| !e.starts_with("prefill:")),
        "私密时不预填：{:?}",
        word.events()
    );
}

#[test]
fn sync_scorers_forget_directly() {
    let word = Recorder {
        keyless: true,
        ..Recorder::default()
    };
    let mut engine = engine().with_word_scorer(Box::new(word.clone()), Some(1.0));
    engine.forget_neural_context();
    assert_eq!(word.events(), ["forget"]);
}

/// 预填排在打分后面时不能把打分顶掉：被顶掉的打分永远不回，引擎会一直以为它在飞。
#[test]
fn prefill_does_not_swallow_a_queued_score() {
    let sentence = Recorder {
        delay: Duration::from_millis(80),
        ..Recorder::default()
    };
    let mut engine =
        engine().with_async_sentence_scorer(Box::new(sentence.clone()), Some(0.5), None, None);
    // 第一条预填占住线程，后面的打分与第二条预填在队列里排着
    engine.set_rescoring_context(Some("上文一".to_owned()));
    let mut paths = vec![path("开饭", -10.0), path("开放", -11.0)];
    engine.rescore_paths(&mut paths, "kaifang");
    assert!(engine.request_rescoring());
    engine.set_rescoring_context(Some("上文二".to_owned()));
    let started = Instant::now();
    while engine.rescoring_in_flight() {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "打分被顶掉了：{:?}",
            sentence.events()
        );
        engine.poll_rescoring();
        std::thread::sleep(Duration::from_millis(2));
    }
    sentence.wait_for("prefill:上文二");
    let stats = engine.last_rescore_stats().expect("打分回来了");
    assert_eq!((stats.reused_tokens, stats.prefix_tokens), (3, 5));
}
```

- [ ] **Step 2: 跑测试，确认编译失败**

Run: `cd $W && cargo test -p qingjian-core prefill`
Expected: 编译错误 `no method named forget_neural_context`、`no field reused_tokens`。

- [ ] **Step 3: `RescoreStats` 加两个字段**

`rescoring/stats.rs` 的 `max_chars` 之后加：

```rust

    /// 这次打分从前缀缓存里复用的 token 数（B4；通变是拼音前缀，知微是前文）。
    pub reused_tokens: usize,

    /// 这次打分的前缀总长（token）。`reused_tokens == prefix_tokens` 是整段命中，0 是整段重算。
    pub prefix_tokens: usize,
```

- [ ] **Step 4: 后台线程支持预填 / 清空任务**

`rescoring/worker.rs` 整文件替换为：

```rust
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
```

- [ ] **Step 5: Engine 侧的预填与清空**

`rescoring/prefill/mod.rs` 写全：

```rust
//! 上文的模型状态跨按键缓存（B4）在 Core 这一侧的两件事：上文变了把预填任务送去后台，
//! 私密输入 / 换学习范围时让模型丢掉缓存的前文与按键状态。模型那一侧见 qingjian-neural 的 `ContextCache`。

#[cfg(test)]
mod tests;

use crate::engine::Engine;

impl Engine {
    /// 当前给模型看的前文送去两个打分线程预填（同一段只送一次，见 `RescoreWorker::prefill`）。
    /// 私密输入不送；同步打分器（CLI）查询时当场算，不用预填。
    pub(super) fn prefill_rescoring_context(&self) {
        if self.private {
            return;
        }
        let context = self.rescoring_context();
        if context.is_empty() {
            return;
        }
        for worker in [&self.rescorer, &self.word_rescorer].into_iter().flatten() {
            worker.prefill(&context);
        }
    }

    /// 让模型丢掉缓存的前文与按键状态：进出私密输入时 Core 自己调；壳换学习范围（换聊天对象）时调。
    pub fn forget_neural_context(&mut self) {
        for worker in [&self.rescorer, &self.word_rescorer].into_iter().flatten() {
            worker.forget();
        }
        for scorer in [&self.sentence_scorer, &self.word_scorer]
            .into_iter()
            .flatten()
        {
            scorer.forget();
        }
    }
}
```

`rescoring/mod.rs` 的 `set_rescoring_context` 改成：

```rust
    /// 壳告知应用里光标前的文本（每次查询前给；应用给不出就 `None`，退回本会话历史）。
    /// 前文变了（上屏、移光标、换输入框）就让后台先把它算进模型（B4 预填）。
    pub fn set_rescoring_context(&mut self, before: Option<String>) {
        self.rescoring_before = before;
        self.prefill_rescoring_context();
    }
```

`poll_rescoring` 里构造 `RescoreStats` 时，`main_ms: …,` 之后加：

```rust
                reused_tokens: scored.reuse.0,
                prefix_tokens: scored.reuse.1,
```

`privacy.rs` 的 `set_private_from` 里 `self.logger.set_muted(combined);` 之后加：

```rust
        // 进出私密都清：进时前文不能留在模型里，出时私密框里敲的按键状态不能带出去（素笺分叉，B4）
        self.forget_neural_context();
```

- [ ] **Step 6: 跑测试，确认通过**

Run: `cd $W && cargo test -p qingjian-core rescoring 2>&1 | grep -E "^test |test result"`
Expected: `prefill::tests::` 下 4 条（`changed_context_is_prefilled_once`、`private_input_forgets_and_does_not_prefill`、`sync_scorers_forget_directly`、`prefill_does_not_swallow_a_queued_score`）通过，原有 `rescoring::tests` 与 `word_rescore::tests` 全部照旧通过。再跑 3 遍排查时序抖动：
`for i in 1 2 3; do cargo test -q -p qingjian-core prefill 2>&1 | grep "test result"; done`

- [ ] **Step 7: 提交**

```bash
cd $W && cargo fmt --all && cargo clippy -p qingjian-core --all-targets -- -D warnings && cargo test -q -p qingjian-core
git add crates/qingjian-core/src/engine/
git commit -m "perf(core): 上文变了后台先预填模型状态，进出私密清掉" -m "set_rescoring_context 前文变了给两条打分线程各送一条预填（同一段只送一次），上屏 / 移光标后第一次重排不再在关键路径上整段重算。
后台线程原来新任务直接顶掉旧任务，预填排在打分后面会把打分顶掉、rescoring_in_flight 永远为真；改成打分与预填各留最新一条、按提交顺序做。
进出私密都让模型丢掉缓存的前文与按键状态；Engine::forget_neural_context 也给壳换学习范围时用。
复用数进 RescoreStats，真机日志看命中率。"
```

---

### Task 6: 知微的分数表跨按键保留

**Files:**
- Modify: `crates/qingjian-core/src/engine/rescoring/word_rescore/mod.rs`（`rescore_first_page` 取 `keys` 那两行；`word_rescoring_pending` 之前加方法）
- Modify: `crates/qingjian-core/src/engine/rescoring/prefill/tests.rs`（追加）

- [ ] **Step 1: 写失败的测试**

在 `prefill/tests.rs` 末尾追加：

```rust
/// 知微不看按键：同一前文下按键变了分数表不清，三次查询里 邮箱 / 油箱 只打一次（看按键的每键都打）。
#[test]
fn keyless_word_scores_survive_keystrokes() {
    for (keyless, expected) in [(true, 1), (false, 3)] {
        let word = Recorder {
            keyless,
            ..Recorder::default()
        };
        let mut engine = engine().with_word_scorer(Box::new(word.clone()), Some(1.0));
        for input in ["youxiang", "youxian", "youxiang"] {
            engine.set_input(input);
            engine.query().unwrap();
        }
        let scored = word
            .events()
            .iter()
            .filter(|e| e.starts_with("score:") && e.contains("油箱"))
            .count();
        assert_eq!(scored, expected, "keyless={keyless}: {:?}", word.events());
    }
}
```

- [ ] **Step 2: 跑测试，确认失败**

Run: `cd $W && cargo test -p qingjian-core keyless_word_scores_survive_keystrokes`
Expected: FAIL，`keyless=true` 那轮 `left: 3, right: 1`（按键进了条件，每键清表重打）。

- [ ] **Step 3: 实现**

`rescore_first_page` 里

```rust
        let keys = self.composition.scope().to_owned();
```
换成
```rust
        // 知微不看按键：按键不进条件，分数表跨按键留着（B4），只有前文变了才清
        let keys = if self.word_scorer_uses_keys() {
            self.composition.scope().to_owned()
        } else {
            String::new()
        };
```

`word_rescoring_pending` 之前加：

```rust
    /// 接着的知微打分器的分数依不依赖按键；没接按依赖算（与旧行为一致）。
    fn word_scorer_uses_keys(&self) -> bool {
        match (&self.word_scorer, &self.word_rescorer) {
            (Some(scorer), _) => scorer.uses_keys(),
            (None, Some(worker)) => worker.uses_keys(),
            (None, None) => true,
        }
    }
```

（`request_word_rescoring` 送出的是 `cache.keys()`、`poll_word_rescoring` 比的也是 `cache.keys()`，两边都是空串，异步结果照常认领；`NeuralCache::MAX_ENTRIES = 1024` 照旧兜底。）

- [ ] **Step 4: 跑测试，确认通过**

Run: `cd $W && cargo test -q -p qingjian-core 2>&1 | grep "test result"`
Expected: 全部 ok（原型 368 passed）。

- [ ] **Step 5: 提交**

```bash
cd $W && cargo fmt --all && cargo clippy -p qingjian-core --all-targets -- -D warnings
git add crates/qingjian-core/src/engine/rescoring/
git commit -m "perf(core): 知微的词级分数表不再每键清" -m "词级重排把 composition.scope() 当分数表条件，每键都变、每键清表，可知微根本不看按键。
uses_keys 为假时按键不进条件，同一前文下同一个词只问模型一次（RefIME LookUpLMScoreInCache 那一层）。"
```

---

### Task 7: 桥的统计列与换人清空

**Files:**
- Modify: `cloud/crates/qingjian-cloud-bridge/src/session/rescore.rs:20-28`
- Modify: `cloud/crates/qingjian-cloud-bridge/src/session/memory/mod.rs`（`switch_layers` 里 `scope_changed()` 之后）
- Modify: `cloud/crates/qingjian-cloud-bridge/src/lib.rs:327`（注释）
- Modify: `cloud/ios/Keyboard/Sources/Engine.swift:47`（注释）

- [ ] **Step 1: 统计串加 `reused=`**

`session/rescore.rs` 的 `rescore_stats` 整个换成：

```rust
    /// 最近一次整句重排的分段耗时，`queue=…;forward=…;main=…;paths=…;max_chars=…;reused=复用/前缀`
    /// （毫秒；`reused` 是拼音前缀从缓存里复用的 token 数）；还没有过返回 `None`。
    pub fn rescore_stats(&self) -> Option<String> {
        self.engine.last_rescore_stats().map(|s| {
            format!(
                "queue={:.1};forward={:.1};main={:.1};paths={};max_chars={};reused={}/{}",
                s.queue_ms,
                s.forward_ms,
                s.main_ms,
                s.paths,
                s.max_chars,
                s.reused_tokens,
                s.prefix_tokens
            )
        })
    }
```

`lib.rs` 第 327 行注释里的 `` `queue=…;forward=…;main=…;paths=…;max_chars=…` `` 改成 `` `queue=…;forward=…;main=…;paths=…;max_chars=…;reused=复用/前缀` ``；`Engine.swift` 第 47 行同样改。

- [ ] **Step 2: 换人时清模型缓存**

`session/memory/mod.rs` 的 `switch_layers` 里：

```rust
        self.engine.learner_mut().scope_changed();
        // 换了聊天对象：模型里缓存的前文与按键状态属于上一个人，丢掉
        self.engine.forget_neural_context();
        self.rebuild_hints();
```

- [ ] **Step 3: cloud 工作区编译、测试**

Run:
```bash
cd $W/cloud && cargo fmt --all --check && cargo clippy -p qingjian-cloud-bridge --all-targets -- -D warnings && cargo test -q -p qingjian-cloud-bridge 2>&1 | grep "test result"
```
Expected: 全部 ok（原型 134 + 各集成测试全绿）。若 `Cargo.lock` 有变动（不应有，没加依赖），一并提交。

- [ ] **Step 4: 提交**

```bash
cd $W && git add cloud/crates/qingjian-cloud-bridge/src cloud/ios/Keyboard/Sources/Engine.swift
git commit -m "feat(cloud): 重排日志多一列 reused，换聊天对象时清模型缓存" -m "真机验 B4 看 reused=复用/前缀：第 2 键起应等于上一键的前缀长。
换人后前文与按键状态属于上一个人，与私密输入同样丢掉。"
```

---

### Task 8: 延迟测量（同一上文连续 10 键，第 2 键起）

仓库已有的 `examples/p2c_bench.rs` 量的是 beam 解码与单步推进，没有「连续按键 / 上屏后第一次」的场景，所以新加一个 example。改前的「整段重算」用每键先 `forget()` 再打分来复现（与改前 `score_p2c` 每键 `prefix_cache(head)` 是同一段计算）。

**Files:**
- Create: `crates/qingjian-neural/examples/context_cache_bench.rs`

- [ ] **Step 1: 写 example**

```rust
//! 前缀缓存（B4）的延迟：同一上文下连续 10 键的重排耗时，缓存留着（现在）对每键清空（等同改前的整段重算）。
//!
//! 通变：拼音每键多一个字母，`score_p2c` 6 条路径；知微：64 字前文上屏两个字后第一次打分，增量 / 预填 / 整段重算三种。
//! 用法：`cargo run --release -p qingjian-neural --example context_cache_bench -- [通变模型] [知微模型]`
//! 缺省 `data/models/hanzhang-tongbian-q8`（iOS 装的 8 位）与 `data/models/hanzhang-zhiwei`。

use std::path::PathBuf;
use std::time::Instant;

use qingjian_neural::CharScorer;

/// 连续敲的 10 键：这串拼音从第 10 个字母敲到第 19 个。
const KEYS: &str = "womenmingtiankaihui";

/// 每键重排的整句路径（iOS 一次重排 6 条）。
const PATHS: [&str; 6] = [
    "我们明天开会",
    "我们明天",
    "我门明天开会",
    "我们名天开会",
    "我们明天看会",
    "沃们明天开会",
];

const ROUNDS: usize = 20;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let tongbian: PathBuf = args
        .next()
        .unwrap_or_else(|| "data/models/hanzhang-tongbian-q8".into())
        .into();
    let zhiwei: PathBuf = args
        .next()
        .unwrap_or_else(|| "data/models/hanzhang-zhiwei".into())
        .into();

    let scorer = CharScorer::load(&tongbian)?;
    println!(
        "== 通变 {}：连续 10 键，每键重排 {} 条 ==",
        tongbian.display(),
        PATHS.len()
    );
    println!("  键   拼音                   缓存留着 ms   每键清空 ms   复用/前缀");
    let mut warm = vec![Vec::new(); 10];
    let mut cold = vec![Vec::new(); 10];
    let mut reuse = [(0, 0); 10];
    for _ in 0..ROUNDS {
        scorer.forget();
        for (key, end) in (10..=KEYS.len()).enumerate() {
            let typed = &KEYS[..end];
            warm[key].push(time(|| {
                scorer.score_p2c(typed, &PATHS).unwrap();
            }));
            reuse[key] = scorer.prefix_reuse();
        }
        for (key, end) in (10..=KEYS.len()).enumerate() {
            let typed = &KEYS[..end];
            scorer.forget();
            cold[key].push(time(|| {
                scorer.score_p2c(typed, &PATHS).unwrap();
            }));
        }
    }
    for key in 0..10 {
        println!(
            "  {:2}   {:20}   {:10.2}   {:10.2}   {}/{}",
            key + 1,
            &KEYS[..10 + key],
            median(&mut warm[key]),
            median(&mut cold[key]),
            reuse[key].0,
            reuse[key].1
        );
    }
    let mut warm_rest: Vec<f64> = warm[1..].iter().flatten().copied().collect();
    let mut cold_rest: Vec<f64> = cold[1..].iter().flatten().copied().collect();
    println!(
        "  第 2 键起中位：缓存留着 {:.2} ms，每键清空 {:.2} ms",
        median(&mut warm_rest),
        median(&mut cold_rest)
    );

    let scorer = CharScorer::load(&zhiwei)?;
    let before: String = "今天下午的会议讨论了输入法的排序问题，大家觉得整句转换还可以再准一些，"
        .repeat(2)
        .chars()
        .take(62)
        .collect();
    let after = format!("{before}我们");
    let words = ["明天", "名天", "明田", "鸣天", "明添", "冥天"];
    println!(
        "\n== 知微 {}：62 字前文上屏两个字后第一次打分 {} 个词 ==",
        zhiwei.display(),
        words.len()
    );
    let mut full = Vec::new();
    let mut incremental = Vec::new();
    let mut prefilled = Vec::new();
    for _ in 0..ROUNDS {
        scorer.forget();
        full.push(time(|| {
            scorer.score(&after, &words).unwrap();
        }));
        scorer.score(&before, &words)?;
        incremental.push(time(|| {
            scorer.score(&after, &words).unwrap();
        }));
        scorer.score(&before, &words)?;
        scorer.prefill(&after)?;
        prefilled.push(time(|| {
            scorer.score(&after, &words).unwrap();
        }));
    }
    println!("  整段重算（改前）  {:6.2} ms", median(&mut full));
    println!("  增量（只算两个字）{:6.2} ms", median(&mut incremental));
    println!("  预填过（关键路径）{:6.2} ms", median(&mut prefilled));
    Ok(())
}

fn time(f: impl FnOnce()) -> f64 {
    let started = Instant::now();
    f();
    started.elapsed().as_secs_f64() * 1e3
}

fn median(samples: &mut [f64]) -> f64 {
    samples.sort_by(f64::total_cmp);
    samples[samples.len() / 2]
}
```

- [ ] **Step 2: 跑**

Run:
```bash
cd $W && cargo run --release -q -p qingjian-neural --example context_cache_bench -- \
  data/models/hanzhang-tongbian-q8 data/models/hanzhang-zhiwei | tee /tmp/b4-bench.txt
```
Expected：第 2–10 键的「复用/前缀」一列是 `11/12`、`12/13` … `19/20`；「第 2 键起中位」缓存留着明显低于每键清空。原型在本机（Apple Silicon，CPU，8 位通变）量到：

```text
  第 2 键起中位：缓存留着 23.55 ms，每键清空 32.29 ms
  整段重算（改前）   45.53 ms
  增量（只算两个字） 24.96 ms
  预填过（关键路径） 18.28 ms
```
把 `/tmp/b4-bench.txt` 的全文贴进 PR 描述。

- [ ] **Step 3: 提交**

```bash
cd $W && cargo fmt --all && cargo clippy -p qingjian-neural --all-targets -- -D warnings
git add crates/qingjian-neural/examples/context_cache_bench.rs
git commit -m "test(neural): 前缀缓存的延迟探针：连续 10 键与上屏后第一次打分" -m "改前的整段重算用每键先 forget 复现；通变缺省用 iOS 装的 8 位模型。"
```

---

### Task 9: 评测、回放与门槛不变

**Files:** 无改动（只读输出）

- [ ] **Step 1: 改后的整句评测与 clean2 回放（带通变）**

```bash
cd $W && cargo build --quiet --release -p qingjian-cli
M=data/models/hanzhang-tongbian/hanzhang-tongbian-small.qjm
target/release/qingjian-cli --config tools/eval/offline.toml --neural $M --eval-text data/eval/sentences.tsv \
  --extra-dict data/generated/dicts/idioms.qj 2>/dev/null > /tmp/b4-eval-new.txt
target/release/qingjian-cli --config tools/eval/offline.toml --neural $M --replay data/eval/input-log-2026-10-04.jsonl \
  --clean2 --extra-dict data/generated/dicts/idioms.qj --misses 0 2>/dev/null > /tmp/b4-replay-new.txt
diff <(grep -v "^查询平均" /tmp/b4-eval-base.txt) <(grep -v "^查询平均" /tmp/b4-eval-new.txt) && echo EVAL_SAME
diff /tmp/b4-replay-base.txt /tmp/b4-replay-new.txt && echo REPLAY_SAME
grep "^查询平均" /tmp/b4-eval-base.txt /tmp/b4-eval-new.txt
```
Expected：`EVAL_SAME` 与 `REPLAY_SAME` 都打出来（逐字相同）；查询平均耗时改后不高于改前（原型 145.7 → 117.1 ms：同步打分时纠错变体之间的拼音前缀也复用上了）。
**判据**：不逐字相同就停下，列出不同的句子与改前 / 改后的分数，查是不是两条路径的神经分只差 1e-4 量级被翻了名次（容差测试说明不了排序在平局附近的行为）；不允许带着差异合并。

- [ ] **Step 2: 门槛脚本（带模型那组）**

```bash
cd $W && tools/release/gate-pass.sh 2>&1 | tee /tmp/b4-gate-new.txt | tail -15
diff <(grep -E "^(none|fusion)" /tmp/b4-gate-base.txt) <(grep -E "^(none|fusion)" /tmp/b4-gate-new.txt) && echo GATE_SAME
```
Expected：`GATE_SAME`（`none` 与 `fusion` 两组每个指标的「现在」与改前同一个数）。门槛本身过不过以改前为准（见 Task 0 Step 3）。

---

### Task 10: 文档：crate-notes 与分叉补丁

**Files:**
- Modify: `docs/notes/crate-notes.md`（`## crates/qingjian-neural` 一节）
- Modify: `cloud/docs/fork-patch.md`（`## 合并上游时` 之前加一节）

- [ ] **Step 1: crate-notes**

在 `docs/notes/crate-notes.md` 的 qingjian-neural 一节，`P2C 教师强制打分 …`那一段之后（「P2C 自由生成实验」之前）加一段：

```markdown
**前缀缓存跨调用复用**（B4，素笺分叉，规格见 [ref-ime-impl/b4-b6-context-cache-and-tiers.md](ref-ime-impl/b4-b6-context-cache-and-tiers.md)）：
`CharScorer` 留一格 `ContextCache`（知微存前文、通变存 `<eos> + 拼音`）。新前缀与旧的完全相同直接用；有公共前缀就 `PrefixCache::truncate`
截到公共前缀，再 `CharLm::extend_cache` 只算多出来的 token（不过输出层）；没有才整段算。打字是追加、退格是截断，上文刚上屏是追加。
占用超过 `CONTEXT_CACHE_MAX_BYTES`（4 MB；8 层 / 448 宽 f32 的 65 个 token 约 1.8 MB）用完就丢。知微的前文按 `context_head` 固定左截到
上下文减 `TAIL_RESERVE`（32），只看前文不看候选，预填（`CharScorer::prefill`）与打分才能算出同一个前缀。增量与整段的数值差：f32 < 1e-4，
8 位 / Metal f16 < 0.08，名次一致（`context_cache/tests.rs`）。Core 一侧：`set_rescoring_context` 前文变了给两条打分线程送预填（`RescoreWorker::prefill`，
同一段只送一次），进出私密与壳换学习范围（`Engine::forget_neural_context`）时清空；后台线程打分与预填各只留最新一条、按提交顺序做，预填不会顶掉打分。
知微 `uses_keys() == false`，词级分数表不拿按键当条件、跨按键保留。`RescoreStats` 的 `reused_tokens / prefix_tokens` 进 iOS 重排日志的 `reused=`。
延迟探针 `cargo run --release -p qingjian-neural --example context_cache_bench`。
```

- [ ] **Step 2: fork-patch**

在 `cloud/docs/fork-patch.md` 的 `## 合并上游时` 之前加：

```markdown
### 前缀 K / V 跨按键复用（B4，Core + neural，各平台都生效）

2026-10-11：iOS 键盘唯一的模型是含章·通变，它的前缀是拼音，每键多一个字母，单格缓存要求完全相等，于是每次重排整段重算；
上屏 / 移光标后第一次重排也在关键路径上整段算。改成按公共前缀截断再增量接续，上文变了后台先预填。规格 docs/notes/ref-ime-impl/b4-b6-context-cache-and-tiers.md。

| 文件 | 改动 | 说明 |
|---|---|---|
| `crates/qingjian-neural/src/context_cache/` | 新目录 | `ContextCache`（同序列直接用 / 公共前缀截断后续算 / 超 4MB 不留）与测试 |
| `crates/qingjian-neural/src/model/mod.rs` | 加 `Clone`、`truncate`、`bytes`、`extend_cache`，加 `mod tests;` | 前缀可截断、可增量接续；`model/tests.rs` 新文件 |
| `crates/qingjian-neural/src/lib.rs` | 加 1 行 | `mod context_cache;` |
| `crates/qingjian-neural/src/scorer.rs` | 换缓存字段，`score` / `score_p2c` 的取缓存段各换 2 行，加 6 个方法 | `context_head` 固定左截（缺省 64 字前文与原截法逐位相同）；`prefill` / `forget` / `prefix_reuse` |
| `crates/qingjian-neural/src/core_scorer.rs` | 加 6 个方法 | 两个实现接新 trait 方法 |
| `crates/qingjian-neural/examples/context_cache_bench.rs` | 新文件 | 延迟探针 |
| `crates/qingjian-core/src/sentence/scorer.rs` | 加 4 个缺省方法 | `prefill` / `forget` / `uses_keys` / `prefix_reuse`，缺省保持旧行为 |
| `crates/qingjian-core/src/engine/rescoring/worker.rs` | 改调度循环，加预填 / 清空任务 | `latest_jobs`：打分与预填各留最新一条、按提交顺序；`Scored.reuse` |
| `crates/qingjian-core/src/engine/rescoring/prefill/` | 新目录 | `prefill_rescoring_context` / `forget_neural_context` 与测试 |
| `crates/qingjian-core/src/engine/rescoring/mod.rs` | 加 1 行 mod、`set_rescoring_context` 加 1 行、统计加 2 行 | 前文变了送预填 |
| `crates/qingjian-core/src/engine/privacy.rs` | 加 1 行 | 进出私密调 `forget_neural_context` |

合并上游时：上游若改了 `CharScorer` 的缓存或 `RescoreWorker` 的调度循环，保留「打分与预填各留最新一条」这条规则（`prefill/tests.rs` 的
`prefill_does_not_swallow_a_queued_score` 守着），再跑 `cargo test -p qingjian-neural context_cache` 与 Task 9 的评测逐字对比。
```

- [ ] **Step 3: 提交**

```bash
cd $W && git add docs/notes/crate-notes.md cloud/docs/fork-patch.md
git commit -m "docs(neural): 记前缀缓存的复用规则、常数与分叉补丁" -m "crate-notes 写缓存规则、两个常数、容差与 Core 侧的预填 / 清空；fork-patch 列出动到的上游文件。用户感知不到，不改 docs/user。"
```

---

### Task 11: iOS 真机 Release 验证（手动，最后做）

**Files:** 无改动

装改前包量一次、装改后包量一次，同一台手机、同一个 App、同样的操作。

- [ ] **Step 1: 改前包**

```bash
cd /Users/liyuqing/sproot/qingjian && git -C . worktree add /tmp/qingjian-b4-base origin/sujian
mkdir -p /tmp/qingjian-b4-base/data && cp -Rc data/generated.shipped /tmp/qingjian-b4-base/data/generated.shipped
ln -s /Users/liyuqing/sproot/qingjian/data/models /tmp/qingjian-b4-base/data/models
/tmp/qingjian-b4-base/cloud/ios/scripts/install-device.sh
```
（缺省 Release；第一次要 Xcode 登录团队账号，见脚本头注释。）

- [ ] **Step 2: 打开 Release 的诊断日志**

Release 包只有 App Group 根下有 `perf-on` 时才写性能日志（`Diagnostics.swift`），键盘要开「完全访问」：

```bash
touch /tmp/perf-on
UDID=$(xcrun devicectl list devices | awk '/iPhone/ {for (i=1;i<=NF;i++) if ($i ~ /^[0-9A-F-]{36}$/) {print $i; exit}}')
xcrun devicectl device copy to --device $UDID --domain-type appGroupDataContainer \
  --domain-identifier group.sujian.synon.ai --source /tmp/perf-on --destination perf-on
```
之后把键盘收起再调出（进程起来时才读开关）。

- [ ] **Step 3: 看日志**

Mac 上打开「控制台」App → 左栏选这台 iPhone → 菜单「操作」勾上「包括简介信息」→ 搜索框填 `subsystem:sujian.keyboard category:perf`，再加关键词 `rescore_ms`，点「开始」。
（命令行替代：`idevicesyslog -m 'rescore_ms='`；看不到 info 级就用控制台。）
每行形如 `rescore_ms=… debounce_ms=… queue=…;forward=…;main=…;paths=…;max_chars=…`，改后的包末尾多 `;reused=复用/前缀`。

- [ ] **Step 4: 操作（改前、改后各做一遍）**

在「备忘录」里：先敲一句并上屏（让光标前有上文，比如「今天下午我们开会讨论了」），然后**一个字母一个字母**敲 `womenmingtiankaihui`，每个字母之间停半秒以上（停键 80 ms 才送重排，每键一行日志），数够 10 行；重复 3 轮。另做一次「上屏后第一次」：敲 `women` 上屏「我们」后立刻敲 `mingtian`，看上屏后第一行。

- [ ] **Step 5: 改后包与判据**

```bash
cd /Users/liyuqing/sproot/qingjian-context-cache && cloud/ios/scripts/install-device.sh
```
重复 Step 3–4。判据：
1. 改后每轮第 2 行起 `reused=` 等于上一行的前缀长（如 `12/13` 的下一行是 `13/14`），整段命中时两数相等；
2. 第 2–10 键的 `forward=` 中位数改后 < 改前（Mac 上原型 32.3 → 23.6 ms，手机上的绝对值会不同，记实测值）；
3. 同一轮里 `footprint_mb=` 的峰值（同一个控制台过滤器去掉 `rescore_ms` 关键词就能看到）不超过改前峰值 + 4 MB；
4. 进一次密码框（如「设置」里的 Wi-Fi 密码）再回备忘录敲字，第一行 `reused=0/…`（私密进出清过缓存）。

把两组 10 行日志与 `footprint_mb` 峰值贴进 PR 描述的「怎么验证的」，写明 iPhone 型号、iOS 版本、App。

- [ ] **Step 6: 收尾**

```bash
git -C /Users/liyuqing/sproot/qingjian worktree remove --force /tmp/qingjian-b4-base
```
`perf-on` 留着只是多写系统日志与 `perf.tsv`，不影响输入。`devicectl` 没有删除文件的子命令；要关诊断就在手机上删掉 App 再装（App Group 容器随最后一个用它的 App 一起清掉）。

---

### Task 12: 推分支、开 PR

**Files:** 无

- [ ] **Step 1: 全量检查**

```bash
cd $W && cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test -q 2>&1 | grep -E "test result|FAILED"
cd $W/cloud && cargo clippy --all-targets -- -D warnings && cargo test -q 2>&1 | grep -E "test result|FAILED"
```
Expected：没有 FAILED。

- [ ] **Step 2: 推送并开 PR（只开到 dayuer/qingjian）**

```bash
cd $W && git push -u origin perf/context-kv-cache
gh pr create --repo dayuer/qingjian --base sujian --head perf/context-kv-cache \
  --title "perf: 上文 / 拼音前缀的模型状态跨按键缓存（B4）" --body-file /tmp/b4-pr.md
```

`/tmp/b4-pr.md` 的内容（把 Task 8 / 9 / 11 的实测贴进去）：

```markdown
## 做了什么
- neural：`ContextCache` 让前缀 K/V 跨调用复用——同序列直接用、公共前缀截断后只算新增 token；通变每键只算新字母，退格零前向；超 4MB 用完就丢。
- core：上文变了后台先预填；进出私密、换聊天对象清空；后台线程打分与预填各留最新一条（预填不再顶掉打分）；知微的词级分数表不再每键清。
- cloud：重排日志多 `reused=复用/前缀`；换人清模型缓存。
- 与规格的差异见计划 cloud/docs/plans/2026-10-11-context-kv-cache.md「与规格的差异」。

## 怎么验证的
- 单测：`cargo test -p qingjian-neural`（两档容差：命中逐位相等；增量 vs 整段 f32 < 1e-4、8 位 < 0.08，名次一致）、`cargo test -p qingjian-core`、cloud bridge 全绿。
- 评测 / 回放（带通变）：改前改后逐字相同（EVAL_SAME / REPLAY_SAME）；gate-pass 两组指标与改前相同（GATE_SAME）。查询平均 <改前> → <改后> ms。
- 延迟探针（context_cache_bench）：<贴 /tmp/b4-bench.txt>
- 真机 Release：<iPhone 型号 / iOS 版本 / 备忘录>，同一上文连续 10 键第 2 键起 forward 中位 <改前> → <改后> ms，reused= 逐键递增；footprint 峰值 <改前> → <改后> MB；进出密码框后 reused=0。
```

不合并；等用户明确说「合 #N」。PR 合了以后：`git -C /Users/liyuqing/sproot/qingjian worktree remove ../qingjian-context-cache`。

---

## Self-Review

- **规格覆盖**：§3.2-1 `ContextCache` → Task 2；§3.2-2 `PrefixCache::truncate` → Task 1；§3.2-3 scorer 换缓存、`score_p2c` 增量、`continue_text` 共用、`prefill`、`CONTEXT_CACHE_MAX_BYTES` → Task 2/3；§3.2-4 trait 缺省方法 → Task 4；§3.2-5 worker 预填任务 → Task 5；§3.2-6 `set_rescoring_context` 下发、词级分数表按 `uses_keys` → Task 5/6；§3.2-7 失效（私密、学习范围）→ Task 5/7；§3.2-8 4 MB 上限 → Task 2；§3.3-2 固定左截 → Task 3；§4-6 `reused=` → Task 5/7；§4-7 crate-notes → Task 10；§4-8 验收 → Task 8/9/11。单测清单 10 条：sync 命中 / 追加 / 退格 / 分叉 / 无公共前缀 / 超预算（Task 2）、P2C 逐键 / 预填（Task 3）、私密清空（Task 5）、词级分数表跨键（Task 6）。
- **不做**：B6、`BEAM_WIDTH` / `RESCORE_PATHS` 可设、`P2c::convert` 接缓存、滑窗按 16 字跳（§3.3-3 的可选增强）。
- **类型一致**：`ContextCache::{new, sync, settle, clear, is_empty, last_reuse}`、`CharScorer::{context_head, sync_prefix, prefill, forget, prefix_reuse, prefix_cached}`、`SentenceScorer::{prefill, forget, uses_keys, prefix_reuse}`、`RescoreWorker::{prefill, forget, uses_keys}`、`Engine::{prefill_rescoring_context, forget_neural_context}`、`RescoreStats::{reused_tokens, prefix_tokens}`、`Scored::reuse` 在各 Task 里同名同签名。
