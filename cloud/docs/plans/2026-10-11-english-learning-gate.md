# 英文学习闸门（D3）实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修上游 #545（分段上屏的残片 `i` / 回车的短串 `sm` 学进 `user-english.tsv` 后抢第一、越选越牢的恶性循环），并补上 todo「个人英文词表没有衰减」：
学之前过闸门、两字母英文要选两次才排第一、退格删净撤销那次学习、30 天衰减、中文多选 3 次英文让到第 5 位、旧词表与云同步合并进来的词也过闸门。

**Architecture:** 判定全在 Core（`qingjian-core/src/engine/learning/english/` 新目录，`impl Engine` 分文件），存储在 `qingjian-learning`（版本头、最后使用日、衰减），
平台层一行不加排序逻辑。上游文件只加挂钩行；`Learner` trait 加 4 个缺省为空的方法（`unlearn_english`、`english_needs_sanitize`、`english_sanitized` 与已有的 `forget_english` 配合），
`ScopedLearner`（云桥）逐个转发。云同步客户端读 `user-english.tsv` 时忽略新加的第三列。

**Tech Stack:** Rust（workspace 根与 `cloud/` 两个工作区）、`cargo test` / `clippy`、`qingjian-cli --replay` / `--eval-text` 回放评测。

规格：`docs/notes/wetype-impl/d3-learning-gate.md`（§4 落点、§6 单测清单）；原型 `docs/notes/wetype-impl/proto/d3-english-gate.diff`；删除判据参考
`docs/notes/wetype-impl/c1-c3-cleanup-offline-space.md` C1 节；分叉已有补丁见 `cloud/docs/fork-patch.md`「中文模式下的英文」一节（`looks_like_pinyin_typo`、`listed_english`、两字母带大写让中文、三字母补全不抢第一——本计划沿用，不重复做）。

本计划的全部代码已在 `origin/sujian`（dd1ce657）的临时工作树里写过一遍并跑通：core 379 条、learning 35 条、cloud 全部测试过，workspace 与 cloud 的 clippy `-D warnings` 干净，回放与评测数字见各 Task。

---

## 前置 / 约定（照抄自仓库规矩，执行前读一遍）

- **执行时开独立工作树**：
  ```bash
  git -C /Users/liyuqing/sproot/qingjian fetch origin
  git -C /Users/liyuqing/sproot/qingjian worktree add ../qingjian-english-gate -b english-gate/d3 origin/sujian
  ```
  所有改动在 `/Users/liyuqing/sproot/qingjian-english-gate` 里做；主检出不动。做完 PR 合了就删工作树（`git worktree remove`）。
- **PR 只开到自己的仓库**：`gh pr create --repo dayuer/qingjian --base sujian`。**永远不往上游 qingjian-team/qingjian 开 PR、推送任何东西。** `gh` 命令一律带 `--repo dayuer/qingjian`。
- **提交信息**：Conventional Commits，`<类型>(<范围>): <中文说明>`，范围用 crate / 壳名（core、learning、cloud、docs…，跨好几处可省略），正文写为什么。
  **不要加 Co-Authored-By 或任何署名尾行**（`.githooks/commit-msg` 会拦格式）。
- **代码组织**：一个类型一个文件；子模块用 `foo/mod.rs` 目录；同前缀兄弟文件收进目录（所以新文件放 `learning/english/{gate,origin,sanitize,retract}.rs`，不叫 `english_gate.rs`）；
  导入写精确路径不用 glob（`extras.rs`、`setup.rs` 存量的 `use super::*` 不专门清扫）；单文件 ≤ 800 行，测试超 200 行放 `tests.rs` / `tests/<主题>.rs`；
  新文件要 `//!` 头；注释中文、只写读代码看不出的约束；不写装饰性分隔注释。详见 `docs/contributing.md` 与 `CLAUDE.md`。
- **提交前**：`cargo fmt --all`、`cargo clippy --all-targets -- -D warnings`；改了 `cloud/` 就在 `cloud/` 下也跑 `cargo fmt --all && cargo clippy --all-targets -- -D warnings`；相关 crate 的 `cargo test -p <crate>`。
- **排序改动要跑回放与评测**：判据见 Task 0 与各 Task 的「回放」步骤。`tools/release/gate-pass.sh` 会往 `data/generated/GATE_PASSED` 写标记，
  工作树里的 `data` 是主检出的软链，**不要直接跑这个脚本**（会改主检出的发版标记）；本计划把它里面的两条命令拆出来单独跑、比改前改后。
- **用户能感知的行为变了，同一提交里改 `docs/user/` 对应页**；改了 crate 实现要点同步 `docs/notes/crate-notes.md`。
- **改了上游已有文件**（`crates/`、`apps/` 下非 `cloud/` 的文件）要在 `cloud/docs/fork-patch.md` 记「分叉补丁」（文件 | 改动 | 说明）。本计划在 Task 1 新开一节，后面每个 Task 往表里加行。
- **最后一个 Task**：推分支、开 PR（`--repo dayuer/qingjian --base sujian`）、PR 描述写「怎么验证的」。**不合并**（只按用户明确指令合并）。
- 这是 Core 逻辑改动、有单测与回放兜底，按 `docs/contributing.md`「外部 PR」不要求真机验证；Task 7 有一个可选的本机冒烟。

## 与规格的差异（以代码为准）

1. **文件位置**：规格写 `learning/english_gate.rs` + `english_origin.rs`、`learning/sanitize.rs`。按「同前缀收进目录」改成 `engine/learning/english/{mod,gate,origin,sanitize,retract}.rs`；
   四个常数放 `learning/english/mod.rs` 而不是上游的 `engine/mod.rs`（新代码放新文件，上游文件少改）。
2. **`EnglishOrigin` 只有两种**：`Raw` 与 `Selected`。规格的 `Candidate` / `SentenceTail` 规则完全相同（都只挡残段），合成一个。
3. **`english_learnable` 是 `impl Engine` 方法**（规格写自由函数 `fn(engine: &Engine, …)`），与 `extras.rs` 等同风格；判定用到的分叉补丁 `looks_like_pinyin_typo` 从 `composing.rs` 搬进 `gate.rs`（`composing.rs` 是上游文件，它在那里本来就是分叉补丁）。
4. **规格的 `english_words()` 不加**：`WordList::entries()` 已经能列出个人词表（全局 `user_english()` + 叠加层 `scoped_user_english()`），不必再给 `Learner` 加方法。
   改加 `english_sanitized(&mut self)`：Engine 清完通知学习器，学习器才把文件标成新版本；否则「随包英文表还没挂上就先落了盘」时会永远跳过清理（iOS 键盘是懒加载英文表的）。
5. **清理的时机**：规格写 `with_learner` / `set_learner` 后。代码里**没有 `set_learner`**，而三个壳与 CLI 都是先 `with_learner` 后 `with_english`（`apps/macos/src/host/init.rs:49/91`、`apps/cli/src/main.rs:221/239`），
   iOS 桥是运行时 `set_english(Some)`。清理要判「在不在随包表」，所以挂在 `with_learner`、`with_english`、`set_english` 三处，随包表没挂上就不清、等它挂上。
6. **云同步合并后再清**：`merge_remote` 由壳经 `engine.learner_mut().merge_remote()` 调（`apps/macos/src/host/cloud/inbox.rs:27`、`cloud/crates/qingjian-cloud-bridge/src/session/cloud.rs:192`），Engine 插不进去。
   新加公开方法 `Engine::merge_remote`（合并 + 只对**新合并进来**的英文词过闸门），两处调用改成它。只清新词，免得别的设备英文模式下有意学的短词每次同步都被清一遍。
7. **撤销还要退选择次数**：选英文候选时 `learner.record(candidate)` 也记了次数（`english_weight` 就是它），只 `unlearn_english` 不够。`LastCommit` 加两个字段：`english` 与 `english_recorded`。
8. **BIP-39 计数**：短词门槛会把三四个字母的 BIP-39 词（`zoo`、`act`）挡在「学」之外，但助记词转私密的连续计数不能断（规格单测清单最后一条）。`take_raw` 里另加一条：
   不在残段、像英文词、在 BIP-39 表里的照常 `note_english_commit`（它们本来就由 `MutedLearner` 挡住不落盘）。
9. **衰减只做 Static**：规格让照搬 C1 的 `RetentionPolicy::STATIC_ENGLISH`。C1 的 Dynamic / `Sweeper` 状态机给中文用户词用，审计里搁置了；这里只做英文需要的三个字段与 `should_drop`，
   游标放在 `FrequencyLearner` 上。`user-english.tsv` 第三列「最后使用日」是 Unix 天数。
10. **云同步客户端要跟着改**（规格没提，必须做）：`cloud/crates/qingjian-cloud-client/src/learning/snapshot.rs::parse_table` 把**最后一列**当次数。加了第三列后每行都会被当坏行跳过，
    同步会以为英文词全删了、往服务器推减量。Task 5 让它对英文表丢掉第三列。
11. **两处已有测试的期望要改**（行为本来就该变）：`gist` 只有 4 个字母、测试里不在随包表 → 不再学，改用 `kubectl`（`english.rs` 两处、`english_fork.rs`、`mnemonic.rs`）；
    `Gd` / `OK` / `MP` 选一次就排第一 → 要两次（`english_fork.rs`、`english.rs`）。
12. **回放**：规格的冻结日志数字复现无误：HEAD 词首选 920/1057、英文 16/21；改后英文 15/21，变的是第 1869 行 `ac`（当时回车学进去的小写 ac，现在不学，`ac` 首选变 啊吃、AC 第 3）。
    (c3) 让位让 `de` → DE、`ni` → Ni、`ba` → BA 这类随包英文词从第 2 位退到第 5 位，命中数不变，第 2239 行 `ba` → 吧 从第 3 名升到第 2 名。

## File Structure

| 文件 | 新 / 改 | 职责 | Task |
|---|---|---|---|
| `crates/qingjian-core/src/engine/learning/english/mod.rs` | 新 | `mod` 声明、re-export、四个常数 | 1（2、3、4、6 加行） |
| `crates/qingjian-core/src/engine/learning/english/origin.rs` | 新 | `EnglishOrigin` | 1 |
| `crates/qingjian-core/src/engine/learning/english/gate.rs` | 新 | `english_learnable`、`raw_english_word`、`looks_like_pinyin_typo`（从 composing.rs 搬来） | 1 |
| `crates/qingjian-core/src/engine/learning/english/sanitize.rs` | 新 | 旧词表清一次、`Engine::merge_remote` | 2 |
| `crates/qingjian-core/src/engine/learning/english/retract.rs` | 新 | 退格删净撤销英文学习 | 4 |
| `crates/qingjian-core/src/engine/learning/mod.rs` | 改 | `mod english;` 与 re-export | 1、3、6 |
| `crates/qingjian-core/src/engine/learning/learner.rs` | 改 | trait 加 `english_needs_sanitize` / `english_sanitized` / `unlearn_english` | 2、4 |
| `crates/qingjian-core/src/engine/learning/muted.rs` | 改 | 转发 `unlearn_english`（与 `learn_english` 对称地挡） | 4 |
| `crates/qingjian-core/src/engine/composing.rs` | 改 | `take_raw` 调闸门；`note_backspace` 删净时撤销 | 1、4 |
| `crates/qingjian-core/src/engine/commit/mod.rs` | 改 | 英文候选、句末英文调闸门；记 `LastCommit.english` | 1、4 |
| `crates/qingjian-core/src/engine/commit/last.rs` | 改 | `LastCommit.english` / `english_recorded` | 4 |
| `crates/qingjian-core/src/engine/prediction/mod.rs` | 改 | `LastCommit` 字面量补两个字段 | 4 |
| `crates/qingjian-core/src/engine/extras.rs` | 改 | (b′) 两字母选两次、(c3) 让到第 5 位 | 3、6 |
| `crates/qingjian-core/src/engine/setup.rs` | 改 | `with_learner` / `with_english` / `set_english` 各一行挂清理 | 2 |
| `crates/qingjian-core/src/engine/tests/english_learning.rs` | 新 | 本计划的引擎单测 | 1–6 |
| `crates/qingjian-core/src/engine/tests/{mod,english,english_fork,mnemonic}.rs` | 改 | 挂新测试文件；改两类期望（差异 11） | 1、3 |
| `crates/qingjian-learning/src/frequency_learner/{mod,tables,learner_impl,tests}.rs` | 改 | 版本头、旧文件标记、多列容忍、撤销、最后使用日 | 2、4、5 |
| `crates/qingjian-learning/src/frequency_learner/sweep.rs` | 新 | 英文词衰减（每学一个看 10 条） | 5 |
| `crates/qingjian-learning/src/retention/{mod,policy}.rs`、`lib.rs` | 新 / 改 | `RetentionPolicy::STATIC_ENGLISH`、`today()` | 5 |
| `cloud/crates/qingjian-cloud-bridge/src/scope/scoped_learner.rs` | 改 | 转发新 trait 方法 | 2、4 |
| `cloud/crates/qingjian-cloud-bridge/src/session/cloud.rs`、`apps/macos/src/host/cloud/inbox.rs` | 改 | 收件箱改调 `Engine::merge_remote` | 2 |
| `cloud/crates/qingjian-cloud-client/src/learning/{snapshot,tests}.rs` | 改 | 同步读 `user-english.tsv` 忽略第三列 | 5 |
| `docs/user/input/english-mode.md`、`docs/user/help/data-and-logs.md` | 改 | 用户文档 | 1–6 |
| `docs/notes/crate-notes.md`、`docs/plan/todo.md`、`docs/plan/roadmap.md`、`cloud/docs/fork-patch.md` | 改 | 实现要点、待办、分叉补丁 | 1–7 |

提交顺序：(a) Task 1 → (d) Task 2 → (b′) Task 3 → (c1) Task 4 → (c2) Task 5 → (c3) Task 6 → 回放与文档收尾 Task 7 → PR Task 8。
(a)(d) 两个提交在前、(b)(c) 四个提交在后，回滚 (b)(c) 任一个 `git revert <sha>` 不碰 (a)(d)。(b) 规格里「选择提权不动」，本计划除 (b′) 外不改 `ranking`。

---

### Task 0: 工作树与改前基线

**Files:** 无改动。

- [ ] **Step 1: 开工作树、接评测数据**

```bash
git -C /Users/liyuqing/sproot/qingjian fetch origin
git -C /Users/liyuqing/sproot/qingjian worktree add ../qingjian-english-gate -b english-gate/d3 origin/sujian
cd /Users/liyuqing/sproot/qingjian-english-gate
ln -s /Users/liyuqing/sproot/qingjian/data data   # data/ 整个不进 git，评测数据与产品数据在主检出
printf '' > /tmp/empty.toml
git config core.hooksPath .githooks
```

- [ ] **Step 2: 编改前的 CLI，记基线**

```bash
cd /Users/liyuqing/sproot/qingjian-english-gate
cargo build --release -p qingjian-cli
CLI=./target/release/qingjian-cli
$CLI --config /tmp/empty.toml --replay data/eval/input-log-2026-10-04.jsonl --misses 0 \
  --replay-details /tmp/d3-before.jsonl 2>/dev/null | grep -E "^(词|整句|英文)"
$CLI --config /tmp/empty.toml --eval-text data/eval/sentences.tsv --misses 0 2>/dev/null | grep -E "^句子"
$CLI --config tools/eval/offline.toml --replay data/eval/input-log-2026-10-04.jsonl --clean2 \
  --extra-dict data/generated/dicts/idioms.qj --misses 0 2>/dev/null | sed -n '/干净口径 2/,/整句 /p' | grep -E "词|整句" | tee /tmp/d3-clean2-before.txt
$CLI --config tools/eval/offline.toml --eval-text data/eval/sentences.tsv \
  --extra-dict data/generated/dicts/idioms.qj --misses 0 2>/dev/null | grep -E "^句子" | tee /tmp/d3-eval-before.txt
```

Expected（2026-10-11 主检出的 data 实测；data 换过的话以你这次跑出来的为准，后面只比前后是否一致）：
```
词       1057 条  首选  87.0%  … 命中数 920 / 1057
整句       165 条  首选  61.8%  … 命中数 102 / 165
英文        21 条  首选  76.2%  前五  95.2% … 平均名次 1.48  命中数 16 / 21
句子  1878 条  首选  31.5%  前三  36.4%  前五  36.4%  整句候选  31.5%  字准确率  76.2%
  词    全部 909/1022 = 88.9% …
  整句   全部 97/138 = 70.3% …
```

- [ ] **Step 3: 记本机日志基线（可选，日志每天在长，只比前后）**

```bash
$CLI --config /tmp/empty.toml --replay "$HOME/Library/Application Support/Qingjian/input-log.jsonl" --misses 0 2>/dev/null \
  | grep -E "^(词|整句|英文)" | tee /tmp/d3-local-before.txt
```

---

### Task 1: (a) 英文学习闸门——残段不学、回车短串不学

**Files:**
- Create: `crates/qingjian-core/src/engine/learning/english/mod.rs`
- Create: `crates/qingjian-core/src/engine/learning/english/origin.rs`
- Create: `crates/qingjian-core/src/engine/learning/english/gate.rs`
- Modify: `crates/qingjian-core/src/engine/learning/mod.rs:10-17`
- Modify: `crates/qingjian-core/src/engine/composing.rs:3-13`（导入）、`:369-392`（`take_raw`）、`:477-482`（删 `looks_like_pinyin_typo`）
- Modify: `crates/qingjian-core/src/engine/commit/mod.rs:6`、`:100-171`、`:239-244`
- Create: `crates/qingjian-core/src/engine/tests/english_learning.rs`
- Modify: `crates/qingjian-core/src/engine/tests/mod.rs:11`、`tests/english.rs:49,326-340`、`tests/english_fork.rs:86-89`、`tests/mnemonic.rs:55-62`
- Docs: `docs/user/input/english-mode.md`、`docs/notes/crate-notes.md`、`cloud/docs/fork-patch.md`

- [ ] **Step 1: 写失败的测试**

`crates/qingjian-core/src/engine/tests/english_learning.rs`（新文件）：

```rust
//! 个人英文词的学习闸门（`learning::english`）：残段不学、短串回车不学（上游 #545 的两条复现路径）。

use super::*;

/// 词库：`wolai` 能选 我啦 留下残段 `i`；`sm` 是 什么 的简拼。
const DICT: &str = "我\two\t90000\n来\tlai\t50000\n我啦\two la\t3000\n我来\two lai\t2000\n什么\tshen me\t9000\n可以\tke yi\t9000\n";

/// 随包英文词表。
const WORDS: &str =
    "I\ti\t6000\nIA\tia\t3000\nSm\tsm\t2000\ngist\tgist\t1000\nkey\tkey\t5120\nOK\tok\t9000\n";

/// 记英文学习的桩：`log` 按顺序记 `learn:词` / `unlearn:词`，选过的候选有次数，学到的英文词进个人词表。
#[derive(Default)]
struct SpyLearner {
    log: Arc<Mutex<Vec<String>>>,
    picked: HashMap<String, u32>,
    english: Vec<(String, u32)>,
    list: Option<WordList>,
}

impl SpyLearner {
    fn rebuild(&mut self) {
        let tsv: String = self
            .english
            .iter()
            .map(|(w, n)| format!("{w}\t{}\t{n}\n", w.to_ascii_lowercase()))
            .collect();
        self.list = WordList::parse(&tsv).ok().filter(|l| !l.is_empty());
    }
}

impl Learner for SpyLearner {
    fn record(&mut self, candidate: &Candidate) {
        *self.picked.entry(candidate.text.clone()).or_default() += 1;
    }

    fn weight(&self, text: &str) -> u32 {
        self.picked.get(text).copied().unwrap_or(0)
    }

    fn unrecord(&mut self, text: &str) {
        if let Some(n) = self.picked.get_mut(text) {
            *n = n.saturating_sub(1);
        }
    }

    fn record_choice(&mut self, input: &str, text: &str) {
        *self.picked.entry(format!("{input}\t{text}")).or_default() += 1;
    }

    fn choice_weight(&self, input: &str, text: &str) -> u32 {
        self.picked
            .get(&format!("{input}\t{text}"))
            .copied()
            .unwrap_or(0)
    }

    fn learn_english(&mut self, word: &str) {
        self.log.lock().unwrap().push(format!("learn:{word}"));
        match self
            .english
            .iter_mut()
            .find(|(w, _)| w.eq_ignore_ascii_case(word))
        {
            Some((_, n)) => *n += 1,
            None => self.english.push((word.to_owned(), 1)),
        }
        self.rebuild();
    }

    fn user_english(&self) -> Option<&WordList> {
        self.list.as_ref()
    }

    fn forget_english(&mut self, word: &str) -> bool {
        let before = self.english.len();
        self.english.retain(|(w, _)| !w.eq_ignore_ascii_case(word));
        self.rebuild();
        before != self.english.len()
    }
}

fn spy_engine() -> (Engine, Arc<Mutex<Vec<String>>>) {
    let learner = SpyLearner::default();
    let log = learner.log.clone();
    let engine = Engine::new(Dictionary::parse(DICT).unwrap())
        .with_english(WordList::parse(WORDS).unwrap())
        .with_learner(Box::new(learner));
    (engine, log)
}

fn find(engine: &Engine, text: &str) -> Candidate {
    engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|c| c.text == text)
        .unwrap_or_else(|| panic!("候选里没有 {text}"))
}

fn raw(engine: &mut Engine, input: &str) {
    engine.set_input(input);
    let _ = engine.query();
    assert_eq!(engine.take_raw(), input);
}

fn learned(log: &Arc<Mutex<Vec<String>>>) -> Vec<String> {
    log.lock().unwrap().clone()
}

#[test]
fn english_picked_from_a_remainder_is_not_learned() {
    // #545 路径一：`wolai` 选 我啦，剩下的 `i` 只有英文 I，空格上屏不该学
    let (mut engine, log) = spy_engine();
    engine.set_input("wolai");
    let wola = find(&engine, "我啦");
    engine.commit(&wola);
    assert_eq!(engine.composition().scope(), "i");
    let i = find(&engine, "I");
    engine.commit(&i);
    assert!(learned(&log).is_empty());
    assert_eq!(engine.learner().weight("I"), 0, "残段里选的英文次数也不记");

    // 直接敲 `i` 选 I 照学
    engine.set_input("i");
    let i = find(&engine, "I");
    engine.commit(&i);
    assert_eq!(learned(&log), ["learn:I"]);
}

#[test]
fn short_raw_strings_are_not_learned() {
    // #545 路径二：`sm` 回车一次就学成英文、下次压过 什么
    let (mut engine, log) = spy_engine();
    raw(&mut engine, "sm");
    raw(&mut engine, "ia");
    assert!(learned(&log).is_empty());
    engine.set_input("sm");
    assert_eq!(texts_of(&engine)[0], "什么");
}

#[test]
fn raw_strings_under_five_letters_must_be_listed() {
    let (mut engine, log) = spy_engine();
    raw(&mut engine, "ial");
    assert!(learned(&log).is_empty(), "三个字母、随包表里没有：不学");
    raw(&mut engine, "gist");
    assert_eq!(learned(&log), ["learn:gist"], "随包表里有：学");
    raw(&mut engine, "wxyzq");
    assert_eq!(
        learned(&log),
        ["learn:gist", "learn:wxyzq"],
        "五个字母、切不成拼音：不在表里也学"
    );
}

#[test]
fn raw_remainder_is_not_learned() {
    let (mut engine, log) = spy_engine();
    engine.set_input("wowxyzq");
    let wo = find(&engine, "我");
    engine.commit(&wo);
    assert_eq!(engine.composition().scope(), "wxyzq");
    assert_eq!(engine.take_raw(), "wxyzq");
    assert!(learned(&log).is_empty());
}

#[test]
fn english_mode_raw_strings_are_learned_as_before() {
    let (mut engine, log) = spy_engine();
    engine.set_english_mode(true);
    engine.set_input("sm");
    assert_eq!(engine.take_raw(), "sm");
    assert_eq!(learned(&log), ["learn:sm"]);
}

#[test]
fn bip39_streak_still_counts_short_raw_words() {
    // BIP-39 词过不了短词门槛也照常计连续命中：六个三字母词连着回车就转私密
    let (mut engine, log) = spy_engine();
    for word in ["zoo", "act", "art", "arm", "box", "bus"] {
        raw(&mut engine, word);
    }
    assert!(engine.is_private());
    assert!(learned(&log).is_empty(), "BIP-39 词不落盘");
}
```

`crates/qingjian-core/src/engine/tests/mod.rs`：在 `mod english_fork;` 下一行加

```rust
mod english_learning;
```

- [ ] **Step 2: 跑测试，确认失败**

Run: `cargo test -p qingjian-core english_learning`
Expected: 4 条 FAILED（`english_picked_from_a_remainder_is_not_learned`、`short_raw_strings_are_not_learned`、`raw_strings_under_five_letters_must_be_listed`、`raw_remainder_is_not_learned`），
`english_mode_raw_strings_are_learned_as_before` 与 `bip39_streak_still_counts_short_raw_words` 通过（它们守的是不能变的行为）。

- [ ] **Step 3: 写闸门**

`crates/qingjian-core/src/engine/learning/english/mod.rs`（新文件）：

```rust
//! 个人英文词的学习闸门：什么样的上屏学进个人英文词表、学错了怎么退。
//! 规则与回放数字见 `docs/notes/wetype-impl/d3-learning-gate.md` 与 `cloud/docs/fork-patch.md`「英文学习闸门」一节。

mod gate;
mod origin;

pub(in crate::engine) use origin::EnglishOrigin;

/// 中文模式回车原样上屏的串至少几个字母才学成英文词（`sm`、`ia` 不学）。
pub(in crate::engine) const MIN_RAW_ENGLISH_LETTERS: usize = 3;

/// 中文模式回车原样上屏的串不到这么多字母时，随包英文词表里要有才学（`ial` 不学，`gist` 学）。
pub(in crate::engine) const MIN_UNLISTED_ENGLISH_LETTERS: usize = 5;
```

`crates/qingjian-core/src/engine/learning/english/origin.rs`（新文件）：

```rust
//! 这次上屏的英文从哪条路来，闸门按它挑规则。

/// 英文上屏的来路。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::engine) enum EnglishOrigin {
    /// 回车原样上屏的字母串。
    Raw,

    /// 选中的英文候选，或整句末尾的英文词。
    Selected,
}
```

`crates/qingjian-core/src/engine/learning/english/gate.rs`（新文件）：

```rust
//! 学不学：中文模式下残段不学、回车串过长度与词表门槛才学；英文模式照旧全学。

use super::{EnglishOrigin, MIN_RAW_ENGLISH_LETTERS, MIN_UNLISTED_ENGLISH_LETTERS};
use crate::correction;
use crate::engine::{Engine, looks_like_english_word, mnemonic};

impl Engine {
    /// `word` 这次上屏要不要记进个人英文词表。`remainder`：同一段拼音里前面已经上屏过词，这是剩下的残段
    /// （`wolai` 选了 我啦 剩下的 `i`），残段不是用户要打的英文。
    pub(in crate::engine) fn english_learnable(
        &self,
        word: &str,
        origin: EnglishOrigin,
        remainder: bool,
    ) -> bool {
        if self.english_mode {
            return origin == EnglishOrigin::Selected || looks_like_english_word(word, true);
        }
        if remainder {
            return false;
        }
        match origin {
            EnglishOrigin::Selected => true,
            EnglishOrigin::Raw => self.raw_english_word(word),
        }
    }

    /// 中文模式回车原样上屏的 `raw` 算不算一个要学的英文词。
    /// 双拼下全部键都能解成完整音节的（`nihc`）不是英文；整段能读成「拼音头 + 英文尾」的（`woxiangxuexirust`）不学：
    /// 那是没给对候选、用户放弃了，学进去会反过来堵掉混输那条路。
    pub(in crate::engine) fn raw_english_word(&self, raw: &str) -> bool {
        let letters = raw.len();
        let listed = self.listed_english(raw);
        looks_like_english_word(raw, false)
            && letters >= MIN_RAW_ENGLISH_LETTERS
            && (listed || letters >= MIN_UNLISTED_ENGLISH_LETTERS)
            && self.decode(raw).is_none_or(|d| !d.is_complete())
            && self.split_english_tail(raw).is_none()
            // 改一处就是两个音节以上的完整拼音（`woilaiceshi` → wolaiceshi）是敲错了的中文；
            // 随包英文词表里有的、BIP-39 词照学（后者落盘由 MutedLearner 挡，这里只为照常计连续命中）
            && (listed || !looks_like_pinyin_typo(raw) || mnemonic::is_bip39_word(raw))
    }
}

/// 原样上屏的字母串改一处（换位、换 / 多 / 少一个字母）就能读成两个音节以上的完整拼音。
fn looks_like_pinyin_typo(raw: &str) -> bool {
    correction::candidates(&raw.to_ascii_lowercase())
        .iter()
        .any(|c| c.segmentation.syllables.len() >= 2 && c.segmentation.incomplete_count() == 0)
}
```

`crates/qingjian-core/src/engine/learning/mod.rs`：

```rust
// 改前
mod forgotten;
mod learner;
mod muted;

pub use forgotten::Forgotten;
pub use learner::{Learner, MAX_LEARNED_COUNT, NoLearner};
pub(super) use muted::MutedLearner;
// 改后
mod english;
mod forgotten;
mod learner;
mod muted;

pub(super) use english::EnglishOrigin;
pub use forgotten::Forgotten;
pub use learner::{Learner, MAX_LEARNED_COUNT, NoLearner};
pub(super) use muted::MutedLearner;
```

- [ ] **Step 4: `take_raw` 改调闸门**

`crates/qingjian-core/src/engine/composing.rs` 导入：

```rust
// 改前
use super::learning::Learner;
…
use crate::composition::Composition;
use crate::correction;
use crate::shortcut;
// 改后
use super::learning::{EnglishOrigin, Learner};
…
use crate::composition::Composition;
use crate::shortcut;
```

`take_raw` 里从 `// 原样上屏的是个英文词（\`gist\`）：记进个人英文词表，下次直接出候选。` 到 `if english_word {` 这一段（含两行之间的注释与 `let english_word = …;`）整个换成：

```rust
        // 原样上屏的是个英文词（`gist`）：记进个人英文词表，下次直接出候选；学不学由闸门定（`learning::english`）
        let remainder = self.chain.same_buffer();
        let english_word = self.english_learnable(&raw, EnglishOrigin::Raw, remainder);
        // BIP-39 词过不了短词门槛也照常计连续命中（助记词转私密输入）；它们本来就不落盘
        if english_word
            || (!remainder
                && mnemonic::is_bip39_word(&raw)
                && looks_like_english_word(&raw, self.english_mode))
        {
```

（后面的 `self.note_english_commit(&raw); } else { self.reset_bip39_streak(); }` 不动。`remainder` 必须在函数末尾 `self.chain.reset()` 之前取，上面的位置满足。）

删掉文件末尾的分叉补丁函数（已搬进 `gate.rs`）：

```rust
/// 分叉补丁：原样上屏的字母串改一处（换位、换 / 多 / 少一个字母）就能读成两个音节以上的完整拼音。
fn looks_like_pinyin_typo(raw: &str) -> bool {
    correction::candidates(&raw.to_ascii_lowercase())
        .iter()
        .any(|c| c.segmentation.syllables.len() >= 2 && c.segmentation.incomplete_count() == 0)
}
```

- [ ] **Step 5: 英文候选与句末英文改调闸门**

`crates/qingjian-core/src/engine/commit/mod.rs`：

```rust
// 改前
use super::learning::Learner;
// 改后
use super::learning::{EnglishOrigin, Learner};
```

在 `commit_with` 里 `if !split { … self.chain.begin_buffer(key); }` 之后、`let mut typos = Vec::new();` 之前插入：

```rust
        // 整句末尾的英文词（我想学好rust 的 rust）过了闸门才学，和英文候选上屏一样
        let sentence_english = sentence_words
            .as_ref()
            .and_then(|words| words.last())
            .filter(|w| {
                is_english_word(w)
                    && self.english_learnable(&w.text, EnglishOrigin::Selected, split)
            })
            .map(|w| w.text.clone());
```

英文分支：

```rust
// 改前
                if candidate.kind == CandidateKind::English {
                    self.learner.record(candidate);
                    self.note_english_commit(&candidate.text);
                }
// 改后
                if candidate.kind == CandidateKind::English {
                    // 残段里选的英文（`wolai` 选了 我啦 剩下的 `i` → I）不学，次数也不记，否则下次它照样抢第一
                    if self.english_learnable(&candidate.text, EnglishOrigin::Selected, split) {
                        self.learner.record(candidate);
                        self.note_english_commit(&candidate.text);
                    } else {
                        self.reset_bip39_streak();
                    }
                }
```

BIP-39 计数那段：

```rust
// 改前
        let noted_english = candidate.kind == CandidateKind::English
            || (candidate.kind == CandidateKind::Sentence
                && sentence_words
                    .as_ref()
                    .and_then(|words| words.last())
                    .is_some_and(is_english_word));
// 改后
        let noted_english = candidate.kind == CandidateKind::English || sentence_english.is_some();
```

整句分支：

```rust
// 改前
                    if let Some(word) = words.last().filter(|w| is_english_word(w)) {
                        self.note_english_commit(&word.text);
                    }
// 改后
                    if let Some(word) = &sentence_english {
                        self.note_english_commit(word);
                    }
```

- [ ] **Step 6: 跑新测试**

Run: `cargo fmt --all && cargo test -p qingjian-core english_learning`
Expected: 6 passed。

- [ ] **Step 7: 改四条已有测试的期望（差异 11：`gist` 4 个字母、测试里不在随包表，按新规则不学）**

`crates/qingjian-core/src/engine/tests/english.rs`：
- `usage_meter_counts_hanzi_words_and_english_words_per_commit` 里 `engine.set_input("gist");` → `engine.set_input("kubectl");`
- `raw_committed_english_words_are_learned_and_come_back_as_candidates` 里从 `// 随包词表里没有 gist：…` 到 `assert_eq!(second.candidates.items[0].text, "gist");` 这一段的 `gist` 全部换成 `kubectl`（注释里两处也换）。

`crates/qingjian-core/src/engine/tests/english_fork.rs` 的 `raw_committed_pinyin_typos_are_not_learned_as_english` 末尾：

```rust
    // 切不成拼音、改一处也不行的照旧学（kubectl）
    engine.set_input("kubectl");
    assert_eq!(engine.take_raw(), "kubectl");
    assert!(english_candidate(&mut engine, "kubectl"));
```

`crates/qingjian-core/src/engine/tests/mnemonic.rs` 的 `bip39_words_are_not_persisted_but_ordinary_english_words_are` 末尾：

```rust
    // "kubectl" 不在表里：普通英文词学习行为不受影响
    commit_raw(&mut engine, "kubectl");
    assert_eq!(
        engine
            .learner()
            .user_english()
            .and_then(|list| list.get("kubectl")),
        Some("kubectl"),
        "表外的普通英文词应该照常学习"
    );
```

（同文件另外两处 `commit_raw(&mut engine, "gist")` 用来「打断连续命中」，不学了照样打断，不改。）

Run: `cargo test -p qingjian-core`
Expected: `test result: ok. 369 passed`。

- [ ] **Step 8: 回放**

```bash
cargo build --release -p qingjian-cli
./target/release/qingjian-cli --config /tmp/empty.toml --replay data/eval/input-log-2026-10-04.jsonl --misses 0 \
  --replay-details /tmp/d3-a.jsonl 2>/dev/null | grep -E "^(词|整句|英文)"
python3 -c "
import json
a=[json.loads(l) for l in open('/tmp/d3-before.jsonl')]
b=[json.loads(l) for l in open('/tmp/d3-a.jsonl')]
[print(x,'\n',y) for x,y in zip(a,b) if x!=y]"
./target/release/qingjian-cli --config /tmp/empty.toml --eval-text data/eval/sentences.tsv --misses 0 2>/dev/null | grep -E "^句子"
```

Expected：词 920/1057、整句 102/165 与 Task 0 一致；英文 **15/21**（前五 90.5%、平均名次 1.50）；逐条差异只有一行：

```
{'line': 1869, 'rank': 1, 'scope': 'ac', 'source': '英文', 'text': 'ac', 'top': ['ac', '∧', '啊吃', '阿昌族', '阿城县']}
{'line': 1869, 'rank': None, 'scope': 'ac', 'source': '英文', 'text': 'ac', 'top': ['啊吃', '阿昌族', 'AC', '∧', '阿城县']}
```

（当时回车学进去的小写 `ac` 现在不学；回车照样能打出 ac。）整句评测与 Task 0 逐字相同。差异多于这一行就停下查。

- [ ] **Step 9: 文档**

`docs/user/input/english-mode.md`「## 记住用户的词」一节，第一段改成：

```markdown
按 `Enter` 原样上屏过的英文词（如 `gist`、`python`）会记入个人词表，下次输入时排在首位。选过的英文候选同样记录。

中文模式下这几种不记，免得没打出来的拼音被当成英文、以后反过来压住中文：

- 不到 3 个字母的（`sm`、`ia`），以及不到 5 个字母、内置英文词表里又没有的（`ial`）；英文模式下照记。
- 一段拼音先选了前面的词、剩下的部分（`wolai` 选「我啦」后剩下的 `i`）。
```

`docs/notes/crate-notes.md`：在「- 中英混输的英文词位置：…缺省关是回放定的（9241 词 / 269 条英文上屏：缺省开英文首选 82.5% → 7.1%）。」这一条之后加一条：

```markdown
- 个人英文词的学习闸门（`engine/learning/english/`，分叉补丁）：`take_raw`、英文候选、句末英文词都先过 `Engine::english_learnable`。
  中文模式下同一段拼音里前面上屏过词（`chain.same_buffer()`）的残段一律不学；回车原样上屏的串不到 `MIN_RAW_ENGLISH_LETTERS`（3）个字母不学、
  不到 `MIN_UNLISTED_ENGLISH_LETTERS`（5）个字母又不在随包英文表的不学，原来的「像英文词 / 双拼解不全 / 不是拼音头 + 英文尾 / 改一处不是拼音」照旧。
  英文模式不受影响。BIP-39 词过不了短词门槛也照常计连续命中。依据与回放：`docs/notes/wetype-impl/d3-learning-gate.md`。
```

同文件 qingjian-learning 段「原样上屏的串要像一个词才学：`MIN_ENGLISH_WORD_LETTERS` = 2 到 `MAX_ENGLISH_WORD_LETTERS` = 15 个字母、切不成完整拼音、」这一行之前加半句，整句变成：

```markdown
  原样上屏的串要像一个词才学（学不学由 Core 的学习闸门定，见上面 qingjian-core 段）：`MIN_ENGLISH_WORD_LETTERS` = 2 到 `MAX_ENGLISH_WORD_LETTERS` = 15 个字母、切不成完整拼音、
```

`cloud/docs/fork-patch.md`：在「## 合并上游时」之前插入新的一节：

```markdown
### 英文学习闸门（Core + learning + 云桥，各平台都生效；2026-10-11，上游 #545）

上游 #545：分段上屏剩下的非拼音残段（`wolai` 选 我啦 剩 `i`）选了英文 I 就学进个人英文词表；短串回车（`sm`）学成英文后压过 什么，越选越牢。
规格 `docs/notes/wetype-impl/d3-learning-gate.md`，计划 `cloud/docs/plans/2026-10-11-english-learning-gate.md`。新代码都在 `engine/learning/english/`，上游文件只加挂钩。

| 文件 | 改动 | 说明 |
|---|---|---|
| `crates/qingjian-core/src/engine/learning/english/` | 新目录 | `english_learnable` 闸门、`EnglishOrigin`、常数 |
| `crates/qingjian-core/src/engine/learning/mod.rs` | 加 2 行 | `mod english;` 与 re-export |
| `crates/qingjian-core/src/engine/composing.rs` | 改约 15 行 | `take_raw` 改调闸门；分叉补丁 `looks_like_pinyin_typo` 搬进 `gate.rs` |
| `crates/qingjian-core/src/engine/commit/mod.rs` | 改约 20 行 | 英文候选、句末英文词改调闸门（残段不学，英文候选的次数也不记） |
| `crates/qingjian-core/src/engine/tests/english_learning.rs` | 新文件 | 闸门的测试 |
| `crates/qingjian-core/src/engine/tests/{english,english_fork,mnemonic}.rs` | 改测试用词 | `gist`（4 个字母、测试里不在随包表）→ `kubectl` |
```

- [ ] **Step 10: 提交**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test -p qingjian-core
git add crates/qingjian-core docs/user/input/english-mode.md docs/notes/crate-notes.md cloud/docs/fork-patch.md
git commit -m "fix(core): 残段与短串回车不再学成个人英文词（上游 #545）" -m "分段上屏剩下的 i / ia 选了英文、sm 回车一次，都会学进 user-english.tsv，下次压过中文、越选越牢。
中文模式下残段一律不学；回车串不到 3 个字母不学、不到 5 个字母又不在随包英文表的不学（WeType 英文模式门槛 3、展示门槛 5）。
冻结日志回放：词 920/1057 不变，英文 16/21 → 15/21，少的是当时回车学进去的 ac。"
```

---

### Task 2: (d) 旧个人英文词表清一次、云同步合并进来的也过闸门

**Files:**
- Create: `crates/qingjian-core/src/engine/learning/english/sanitize.rs`
- Modify: `crates/qingjian-core/src/engine/learning/english/mod.rs`（`mod sanitize;`）
- Modify: `crates/qingjian-core/src/engine/learning/learner.rs:112-116`（`forget_english` 之后）
- Modify: `crates/qingjian-core/src/engine/setup.rs:399-406`、`:477-481`
- Modify: `crates/qingjian-learning/src/frequency_learner/mod.rs:31-32`、`:81-82`
- Modify: `crates/qingjian-learning/src/frequency_learner/tables.rs:10-13`、`:22-40`、`:64-75`
- Modify: `crates/qingjian-learning/src/frequency_learner/learner_impl.rs:214`（`merge_remote` 之前）
- Modify: `crates/qingjian-learning/src/frequency_learner/tests.rs`（末尾追加）
- Modify: `cloud/crates/qingjian-cloud-bridge/src/scope/scoped_learner.rs:215`（`merge_remote` 之前）
- Modify: `cloud/crates/qingjian-cloud-bridge/src/session/cloud.rs:192`、`apps/macos/src/host/cloud/inbox.rs:27`
- Test: `crates/qingjian-core/src/engine/tests/english_learning.rs`
- Docs: `docs/user/input/english-mode.md`、`docs/user/help/data-and-logs.md`、`docs/notes/crate-notes.md`、`cloud/docs/fork-patch.md`

- [ ] **Step 1: 写失败的测试（learning）**

`crates/qingjian-learning/src/frequency_learner/tests.rs` 末尾追加：

```rust
/// 没有版本头的旧 `user-english.tsv` 要 Engine 按学习闸门清一次；清过后落盘写版本头，再读就不用清了。
#[test]
fn legacy_english_file_asks_for_one_sanitize() {
    let dir = std::env::temp_dir().join(format!("qingjian-english-legacy-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("user.tsv");
    std::fs::write(
        dir.join(USER_ENGLISH_FILE),
        "# 青简个人英文词：词\\t次数\nsm\t3\ngist\t2\n",
    )
    .unwrap();

    let mut learner = FrequencyLearner::from_path(&path).unwrap();
    assert!(learner.english_needs_sanitize());
    assert!(learner.forget_english("sm"));
    learner.english_sanitized();
    assert!(!learner.english_needs_sanitize());
    learner.flush();

    let saved = std::fs::read_to_string(dir.join(USER_ENGLISH_FILE)).unwrap();
    assert!(saved.starts_with(USER_ENGLISH_VERSION));
    let reloaded = FrequencyLearner::from_path(&path).unwrap();
    assert!(!reloaded.english_needs_sanitize());
    assert_eq!(reloaded.english_count(), 1);
    std::fs::remove_dir_all(dir).unwrap();
}

/// 清过但一条都没删：也要落一次盘写上版本头，否则下次启动又清一遍。
#[test]
fn sanitizing_without_drops_still_writes_the_version() {
    let dir = std::env::temp_dir().join(format!("qingjian-english-clean-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("user.tsv");
    std::fs::write(dir.join(USER_ENGLISH_FILE), "gist\t2\n").unwrap();

    let mut learner = FrequencyLearner::from_path(&path).unwrap();
    learner.english_sanitized();
    assert!(learner.has_unsaved());
    learner.flush();
    assert!(!FrequencyLearner::from_path(&path).unwrap().english_needs_sanitize());
    std::fs::remove_dir_all(dir).unwrap();
}

/// 新版本多写的列（最后使用日）旧版本读得懂：只取前两列。
#[test]
fn extra_english_columns_are_ignored() {
    let learner = {
        let mut learner = FrequencyLearner::default();
        learner.load_english("# 青简个人英文词 v2：词\\t次数\\t最后使用日\ngist\t2\t20372\n");
        learner
    };
    assert_eq!(learner.english_count(), 1);
    assert!(!learner.english_needs_sanitize());
}
```

Run: `cargo test -p qingjian-learning english`
Expected: 编译失败（`english_needs_sanitize` / `english_sanitized` / `USER_ENGLISH_VERSION` 不存在）。

- [ ] **Step 2: `Learner` 加两个缺省方法**

`crates/qingjian-core/src/engine/learning/learner.rs`，在 `fn forget_english(&mut self, _word: &str) -> bool { false }` 之后、`/// 合并别的设备的学习增量…` 之前插入：

```rust
    /// 个人英文词表还是学习闸门之前的旧版本（`user-english.tsv` 没有版本头）：Engine 挂上随包英文词表后按闸门规则清一次，
    /// 清完调 [`Self::english_sanitized`]。包装层要转发。
    fn english_needs_sanitize(&self) -> bool {
        false
    }

    /// Engine 清过个人英文词表了：下次落盘写成新版本，之后不再清。
    fn english_sanitized(&mut self) {}
```

- [ ] **Step 3: `FrequencyLearner` 认版本头**

`crates/qingjian-learning/src/frequency_learner/mod.rs`，`const USER_ENGLISH_FILE` 之后加：

```rust
/// 个人英文词表版本头的开头。没有它的是学习闸门之前的旧文件，Engine 会按闸门规则清一次（`Learner::english_needs_sanitize`）。
const USER_ENGLISH_VERSION: &str = "# 青简个人英文词 v2";
```

结构体 `english_dirty: bool,` 字段之后加：

```rust

    /// 个人英文词是从没有版本头的旧文件读的，还没按学习闸门清过。
    english_legacy: bool,
```

`crates/qingjian-learning/src/frequency_learner/tables.rs` 导入：

```rust
use super::{
    FrequencyLearner, USER_CHOICES_FILE, USER_ENGLISH_FILE, USER_ENGLISH_VERSION, USER_NGRAM_FILE,
    USER_TYPOS_FILE, USER_WORD_FREQUENCY, USER_WORDS_FILE, data_lines, parse_count,
    parse_counted_pair,
};
```

`load_english` 整个换成：

```rust
    /// 从 `词\t次数` 读个人英文词，返回跳过的坏行数。后面多出的列不认识就不管（新版本加的列，回退到旧版本时照读）。
    pub(super) fn load_english(&mut self, source: &str) -> usize {
        let mut skipped = 0;
        for line in data_lines(source) {
            let mut columns = line.split('\t');
            let Some((word, count)) = columns
                .next()
                .zip(columns.next())
                .and_then(|(word, count)| Some((word, parse_count(count)?)))
            else {
                skipped += 1;
                continue;
            };
            if count > 0 {
                self.english
                    .insert(word.to_ascii_lowercase(), (word.to_owned(), count));
            }
        }
        self.english_legacy =
            !self.english.is_empty() && !source.trim_start().starts_with(USER_ENGLISH_VERSION);
        self.rebuild_english();
        skipped
    }
```

`save_english_to` 里的表头行：

```rust
// 改前
            writeln!(file, "# 青简个人英文词：词\\t次数")?;
// 改后
            writeln!(file, "{USER_ENGLISH_VERSION}：词\\t次数")?;
```

`crates/qingjian-learning/src/frequency_learner/learner_impl.rs`，`fn merge_remote` 之前加：

```rust
    fn english_needs_sanitize(&self) -> bool {
        self.english_legacy
    }

    fn english_sanitized(&mut self) {
        if self.english_legacy {
            self.english_legacy = false;
            self.english_dirty = true;
        }
    }
```

Run: `cargo test -p qingjian-learning`
Expected: 全部通过（新加 3 条）。

- [ ] **Step 4: 写失败的测试（引擎）**

`crates/qingjian-core/src/engine/tests/english_learning.rs`：

`SpyLearner` 结构体加字段（`list` 之后）：

```rust
    legacy: bool,
```

`impl SpyLearner` 里 `fn rebuild` 之前加：

```rust
    /// 读进来一份没有版本头的旧词表。
    fn legacy(words: &[&str]) -> Self {
        let mut learner = Self {
            english: words.iter().map(|w| ((*w).to_owned(), 1)).collect(),
            legacy: true,
            ..Self::default()
        };
        learner.rebuild();
        learner
    }
```

`impl Learner for SpyLearner` 末尾（`forget_english` 之后）加：

```rust
    fn english_needs_sanitize(&self) -> bool {
        self.legacy
    }

    fn english_sanitized(&mut self) {
        self.legacy = false;
    }

    /// 只认 `english\tadd\t词\t次数` 一种行。
    fn merge_remote(&mut self, inbox: &str) -> usize {
        let mut merged = 0;
        for line in inbox.lines() {
            if let ["english", "add", word, _] = line.split('\t').collect::<Vec<_>>()[..] {
                self.english.push((word.to_owned(), 1));
                merged += 1;
            }
        }
        self.rebuild();
        merged
    }
```

文件末尾追加：

```rust
fn personal(engine: &Engine) -> Vec<String> {
    engine
        .learner()
        .user_english()
        .map_or_else(Vec::new, |list| {
            list.entries().map(|(_, word, _)| word.to_owned()).collect()
        })
}

#[test]
fn legacy_english_list_is_sanitized_once_the_bundled_list_is_there() {
    let learner = SpyLearner::legacy(&["sm", "ia", "ial", "OK", "gist", "kubectl"]);
    // 壳先装学习器、后挂随包英文表：挂表那一刻才清
    let engine = Engine::new(Dictionary::parse(DICT).unwrap()).with_learner(Box::new(learner));
    assert_eq!(personal(&engine).len(), 6, "没有随包表判不了，不清");
    let engine = engine.with_english(WordList::parse(WORDS).unwrap());
    assert_eq!(personal(&engine), ["gist", "kubectl", "OK"]);
    assert!(!engine.learner().english_needs_sanitize());
}

#[test]
fn current_english_list_is_left_alone() {
    let mut learner = SpyLearner::legacy(&["sm"]);
    learner.legacy = false;
    let engine = Engine::new(Dictionary::parse(DICT).unwrap())
        .with_english(WordList::parse(WORDS).unwrap())
        .with_learner(Box::new(learner));
    assert_eq!(personal(&engine), ["sm"]);
}

#[test]
fn english_merged_from_other_devices_goes_through_the_gate() {
    let (mut engine, _) = spy_engine();
    let merged = engine.merge_remote("english\tadd\tia\t1\nenglish\tadd\tkubectl\t1\n");
    assert_eq!(merged, 2);
    assert_eq!(personal(&engine), ["kubectl"]);
}
```

Run: `cargo test -p qingjian-core english_learning`
Expected: 编译失败（`Engine::merge_remote` 不存在）。

- [ ] **Step 5: 写清理**

`crates/qingjian-core/src/engine/learning/english/sanitize.rs`（新文件）：

```rust
//! 已经学进个人英文词表的碎片按闸门规则清掉：旧词表加载时清一次，云同步合并进来的新词每次都过一遍。

use crate::engine::Engine;
use crate::engine::learning::Learner;

impl Engine {
    /// 个人英文词表是闸门之前的旧版本（[`crate::Learner::english_needs_sanitize`]）且随包英文词表已挂上：
    /// 按回车上屏的规则过一遍，不过的删掉，然后告诉学习器清过了。随包表没挂上时不清（判不了「在不在表里」），等它挂上再来。
    pub(in crate::engine) fn sanitize_user_english_once(&mut self) {
        if self.english.is_none() || !self.learner.inner().english_needs_sanitize() {
            return;
        }
        let words = self.personal_english_words();
        let dropped = self.drop_unlearnable_english(&words);
        self.learner.inner_mut().english_sanitized();
        tracing::info!(dropped, total = words.len(), "清理个人英文词");
    }

    /// 合并别的设备的学习增量（青简 Cloud 收件箱的文本），合并进来的新英文词同样过闸门，返回合并了几条。
    /// 壳的收件箱挂钩调它，而不是直接调 [`crate::Learner::merge_remote`]：别的设备学进去的碎片不能借同步带回来。
    pub fn merge_remote(&mut self, inbox: &str) -> usize {
        let before = self.personal_english_words();
        let merged = self.learner_mut().merge_remote(inbox);
        let fresh: Vec<String> = self
            .personal_english_words()
            .into_iter()
            .filter(|word| !before.contains(word))
            .collect();
        if !fresh.is_empty() && self.english.is_some() {
            let dropped = self.drop_unlearnable_english(&fresh);
            tracing::debug!(dropped, fresh = fresh.len(), "合并进来的个人英文词过闸门");
        }
        merged
    }

    /// 个人英文词（全局与叠加层两份），按学到的写法。
    fn personal_english_words(&self) -> Vec<String> {
        let scoped = self.learner.inner().scoped_user_english();
        scoped
            .into_iter()
            .chain(self.learner.user_english())
            .flat_map(|list| list.entries().map(|(_, word, _)| word.to_owned()))
            .collect()
    }

    /// 删掉 `words` 里过不了闸门的，返回删了几个。文件不记来路，一律按回车上屏的规则判（最严的那条）；
    /// 与随包词表写法完全一致的不删（`OK`、`I`），它们排第几由两字母门槛管。
    fn drop_unlearnable_english(&mut self, words: &[String]) -> usize {
        let unlearnable: Vec<&String> = words
            .iter()
            .filter(|word| {
                let code = word.to_ascii_lowercase();
                let exact = self
                    .english
                    .as_ref()
                    .is_some_and(|list| list.get(&code) == Some(word.as_str()));
                !exact && !self.raw_english_word(&code)
            })
            .collect();
        for word in &unlearnable {
            self.learner.forget_english(word);
        }
        if !unlearnable.is_empty() {
            self.forget_span_cache();
        }
        unlearnable.len()
    }
}
```

`crates/qingjian-core/src/engine/learning/english/mod.rs`：`mod origin;` 之后加 `mod sanitize;`。

`crates/qingjian-core/src/engine/setup.rs` 三处挂钩：

```rust
// 改前
    pub fn set_english(&mut self, words: Option<WordList>) {
        self.english = words;
    }

    pub fn with_english(mut self, words: WordList) -> Self {
        self.english = Some(words);
        self
    }
// 改后
    pub fn set_english(&mut self, words: Option<WordList>) {
        self.english = words;
        self.sanitize_user_english_once();
    }

    pub fn with_english(mut self, words: WordList) -> Self {
        self.english = Some(words);
        self.sanitize_user_english_once();
        self
    }
```

```rust
// 改前
    pub fn with_learner(mut self, learner: Box<dyn Learner>) -> Self {
        self.learner.replace(learner);
        self.forget_span_cache();
        self
    }
// 改后
    pub fn with_learner(mut self, learner: Box<dyn Learner>) -> Self {
        self.learner.replace(learner);
        self.forget_span_cache();
        self.sanitize_user_english_once();
        self
    }
```

Run: `cargo fmt --all && cargo test -p qingjian-core`
Expected: `test result: ok. 372 passed`。

- [ ] **Step 6: 两个收件箱挂钩改调 `Engine::merge_remote`，云桥转发新方法**

`apps/macos/src/host/cloud/inbox.rs` 与 `cloud/crates/qingjian-cloud-bridge/src/session/cloud.rs` 各一行：

```rust
// 改前
        let applied = self.engine.learner_mut().merge_remote(&text);
// 改后
        let applied = self.engine.merge_remote(&text);
```

`cloud/crates/qingjian-cloud-bridge/src/scope/scoped_learner.rs`，`fn merge_remote` 之前加：

```rust
    // 旧词表只有全局那份：叠加层的个人英文词是闸门之后才有的（2026-10-05 起），清理照常经 forget_english 连叠加层一起删
    fn english_needs_sanitize(&self) -> bool {
        self.global.english_needs_sanitize()
    }

    fn english_sanitized(&mut self) {
        self.global.english_sanitized();
    }
```

Run:
```bash
cargo check -p qingjian-macos
cd cloud && cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test -p qingjian-cloud-bridge && cd ..
```
Expected: 都通过。

- [ ] **Step 7: 文档**

`docs/user/input/english-mode.md`「## 记住用户的词」上一步加的列表之后加一段：

```markdown
升级后第一次启动会按上面的规则把以前记下的个人英文词清理一遍；从别的设备同步来的英文词也按同样的规则过一遍。
```

`docs/user/help/data-and-logs.md` 表格里这一行：

```markdown
| `user-english.tsv` | 原样上屏过、选过的英文词 | 英文词 |
```

`docs/notes/crate-notes.md`：Task 1 加的「个人英文词的学习闸门」那条末尾追加一行：

```markdown
  旧词表（`user-english.tsv` 没有 `# 青简个人英文词 v2` 版本头，`Learner::english_needs_sanitize`）在随包英文表挂上时（`with_learner` / `with_english` / `set_english`）按回车规则清一次，
  与随包写法完全一致的保留，清完 `english_sanitized` 让学习器下次落盘写版本头；壳的云收件箱改调 `Engine::merge_remote`，只对新合并进来的英文词过闸门。
```

`cloud/docs/fork-patch.md`「英文学习闸门」一节的表里追加：

```markdown
| `crates/qingjian-core/src/engine/learning/learner.rs` | 加 2 个缺省方法 | `english_needs_sanitize` 缺省 `false`、`english_sanitized` 缺省什么都不做 |
| `crates/qingjian-core/src/engine/setup.rs` | 加 3 行 | `with_learner` / `with_english` / `set_english` 后各调一次 `sanitize_user_english_once` |
| `crates/qingjian-learning/src/frequency_learner/{mod,tables,learner_impl}.rs` | 加约 25 行 | `user-english.tsv` 版本头 `# 青简个人英文词 v2`、旧文件标记、多出的列不管 |
| `apps/macos/src/host/cloud/inbox.rs`、`cloud/…/session/cloud.rs` | 改 1 行 | 收件箱改调 `Engine::merge_remote`（合并后新英文词过闸门） |
| `cloud/…/scope/scoped_learner.rs` | 加 2 个转发 | 只看全局层 |
```

- [ ] **Step 8: 提交**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test -p qingjian-core -p qingjian-learning
git add crates apps/macos/src/host/cloud/inbox.rs cloud/crates/qingjian-cloud-bridge docs/user docs/notes/crate-notes.md cloud/docs/fork-patch.md
git commit -m "fix: 旧个人英文词表按学习闸门清一次，云同步合并进来的也过闸门" -m "已经被 #545 污染的用户（本机 89 条里 ≤2 字母写法与随包不同的 8 条、3–4 字母不在随包表的 5 条）光改学习入口不会自愈。
user-english.tsv 加版本头，没有头的旧文件在随包英文表挂上时按回车规则清一次；随包表懒加载（iOS）时等它挂上再清。
别的设备学进去的碎片不能借同步带回来，收件箱改调 Engine::merge_remote，只对新合并进来的词过闸门。"
```

---

### Task 3: (b′) 两个字母的英文词要选两次才排第一

**Files:**
- Modify: `crates/qingjian-core/src/engine/learning/english/mod.rs`（加常数）
- Modify: `crates/qingjian-core/src/engine/learning/mod.rs`（re-export）
- Modify: `crates/qingjian-core/src/engine/extras.rs:80-83`（doc 注释）、`:110-121`
- Modify: `crates/qingjian-core/src/engine/tests/english.rs:160-169`、`tests/english_fork.rs:35-57`
- Test: `crates/qingjian-core/src/engine/tests/english_learning.rs`
- Docs: `docs/user/input/english-mode.md`、`docs/notes/crate-notes.md`、`cloud/docs/fork-patch.md`

- [ ] **Step 1: 写失败的测试**

`english_learning.rs` 末尾追加：

```rust
fn pick(engine: &mut Engine, input: &str, text: &str) {
    engine.set_input(input);
    let candidate = find(engine, text);
    engine.commit(&candidate);
}

#[test]
fn two_letter_english_needs_two_picks_to_lead() {
    let (mut engine, _) = spy_engine();
    pick(&mut engine, "sm", "Sm");
    engine.set_input("sm");
    assert_eq!(texts_of(&engine)[..2], ["什么", "Sm"]);
    pick(&mut engine, "sm", "Sm");
    engine.set_input("sm");
    assert_eq!(texts_of(&engine)[0], "Sm");

    // 上游「选过 OK 的用户下次还是它」照旧，只是要两次
    pick(&mut engine, "ok", "OK");
    pick(&mut engine, "ok", "OK");
    engine.set_input("ok");
    assert_eq!(texts_of(&engine)[0], "OK");
}

#[test]
fn lowercase_learned_short_word_is_judged_by_the_bundled_spelling() {
    // 闸门之前回车学进去的小写 `sm`：大小写按随包的 Sm 判，选过一次也不抢 什么
    let mut learner = SpyLearner::legacy(&["sm"]);
    learner.legacy = false;
    learner.picked.insert("sm".to_owned(), 1);
    let mut engine = Engine::new(Dictionary::parse(DICT).unwrap())
        .with_english(WordList::parse(WORDS).unwrap())
        .with_learner(Box::new(learner));
    engine.set_input("sm");
    assert_eq!(texts_of(&engine)[0], "什么");
}
```

Run: `cargo test -p qingjian-core english_learning`
Expected: 这两条 FAILED（选一次 Sm 就排第一；小写 sm 排第一）。

- [ ] **Step 2: 实现**

`learning/english/mod.rs` 末尾加：

```rust

/// 两个字母的英文词（`sm` → Sm、`ok` → OK）选过这么多次才排第一：选一次就翻掉 什么 / 哦，误选一次就恶性循环。
pub(in crate::engine) const SHORT_ENGLISH_FIRST_PICKS: u32 = 2;
```

`learning/mod.rs`：

```rust
// 改前
pub(super) use english::EnglishOrigin;
// 改后
pub(super) use english::{EnglishOrigin, SHORT_ENGLISH_FIRST_PICKS};
```

`extras.rs` 的 `insert_english`：

```rust
// 改前
        let short_acronym = english_weight == 0
            && text.len() <= 2
            && word.is_some_and(|word| word.chars().all(|c| c.is_ascii_uppercase()));
        let short_capitalized = english_weight == 0
            && text.len() <= 2
            && word.is_some_and(|word| word.chars().any(|c| c.is_ascii_uppercase()));
// 改后
        // 分叉补丁：要选过 [`learning::SHORT_ENGLISH_FIRST_PICKS`] 次才不让，大小写按随包表的写法判
        //（小写学进去的 `sm` 也按随包的 Sm 算）。见 `learning::english`
        let shape = self
            .english
            .as_ref()
            .and_then(|list| list.get(text))
            .or(word);
        let picked_enough = english_weight >= learning::SHORT_ENGLISH_FIRST_PICKS;
        let short_acronym = !picked_enough
            && text.len() <= 2
            && shape.is_some_and(|word| word.chars().all(|c| c.is_ascii_uppercase()));
        let short_capitalized = !picked_enough
            && text.len() <= 2
            && shape.is_some_and(|word| word.chars().any(|c| c.is_ascii_uppercase()));
```

同函数 doc 注释里这一行：

```rust
// 改前
    /// 例外只管**没选过**的英文词：用户选过的照旧排第一（选过 OK，下次敲 `ok` 还是 OK 在前）。
// 改后
    /// 例外只管选得还不够的英文词：分叉补丁要选过两次（`learning::SHORT_ENGLISH_FIRST_PICKS`），之后照旧排第一（选过 OK，下次敲 `ok` 还是 OK 在前）。
```

（`extras.rs` 顶部是存量的 `use super::*;`，`learning` 模块经它可见，不用加导入。）

- [ ] **Step 3: 改两条已有测试的期望（差异 11）**

`tests/english.rs` 的 `learned_english_word_keeps_first_place_over_the_two_letter_acronym_rule` 末尾：

```rust
// 改前
    // 选过一次 OK：规则不再压过学习记录，`ok` 回到英文第一。
    pick(&mut engine, "ok", "OK");
    engine.set_input("ok");
    assert_eq!(texts_of(&engine)[0], "OK");

    // `mp` 同理：选过 MP 之后它也从中文切回英文第一。
    pick(&mut engine, "mp", "MP");
    engine.set_input("mp");
    assert_eq!(texts_of(&engine)[0], "MP");
// 改后
    // 分叉补丁（学习闸门）：选过一次还不够，两次才不压过学习记录，`ok` 回到英文第一。
    pick(&mut engine, "ok", "OK");
    engine.set_input("ok");
    assert_eq!(texts_of(&engine)[0], "哦");
    pick(&mut engine, "ok", "OK");
    engine.set_input("ok");
    assert_eq!(texts_of(&engine)[0], "OK");

    // `mp` 同理：选过两次 MP 之后它也从中文切回英文第一。
    pick(&mut engine, "mp", "MP");
    pick(&mut engine, "mp", "MP");
    engine.set_input("mp");
    assert_eq!(texts_of(&engine)[0], "MP");
```

`tests/english_fork.rs` 的 `two_letter_mixed_case_english_yields_to_chinese_until_picked`：首行注释里「选过一次之后才排第一」改成「选过两次之后才排第一」；

```rust
// 改前
    engine.commit(&gd);
    engine.set_input("gd");
    assert_eq!(texts_of(&engine)[0], "Gd");
// 改后
    engine.commit(&gd);
    engine.set_input("gd");
    assert_eq!(texts_of(&engine)[0], "该地", "两个字母的英文选一次还不够（学习闸门）");
    engine.commit(&gd);
    engine.set_input("gd");
    assert_eq!(texts_of(&engine)[0], "Gd");
```

Run: `cargo fmt --all && cargo test -p qingjian-core`
Expected: `test result: ok. 374 passed`。

- [ ] **Step 4: 回放**

同 Task 1 Step 8 的命令（`--replay-details /tmp/d3-b.jsonl`，比 `/tmp/d3-before.jsonl`）。
Expected：数字与 Task 1 完全相同（词 920/1057、英文 15/21），逐条差异仍只有第 1869 行；整句评测不变。
（冷启动回放里没有两字母英文被选过一次的场景，这条规则只在已有学习记录时起作用。）

- [ ] **Step 5: 文档**

`docs/user/input/english-mode.md`「## 记住用户的词」Task 2 那段之后加：

```markdown
两个字母的英文词（`sm` → Sm、`ok` → OK）要选过两次才排到中文前面，误选一次不会把「什么」「哦」挤下去。
```

`docs/notes/crate-notes.md`「个人英文词的学习闸门」那条末尾追加一行：

```markdown
  两个字母的英文词在 `insert_english` 里要选过 `SHORT_ENGLISH_FIRST_PICKS`（2）次才不给中文让位，全大写 / 带大写按随包写法判（小写学进去的 `sm` 按随包的 Sm 算）。
```

`cloud/docs/fork-patch.md`「英文学习闸门」表里追加：

```markdown
| `crates/qingjian-core/src/engine/extras.rs` | 改约 12 行 | 两字母英文让中文的条件从「没选过」改成「选过不到 2 次」，大小写按随包写法判 |
| `crates/qingjian-core/src/engine/tests/{english,english_fork}.rs` | 改期望 | OK / MP / Gd 要选两次 |
```

- [ ] **Step 6: 提交**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test -p qingjian-core
git add crates/qingjian-core docs/user/input/english-mode.md docs/notes/crate-notes.md cloud/docs/fork-patch.md
git commit -m "fix(core): 两个字母的英文词要选两次才排到中文前面" -m "原来选过一次就绕过「两字母让中文」，误选一次 Sm / Gd 就压过 什么 / 该地，之后空格又记一次，越选越牢。
对应 WeType「短英文词不靠一次选择上位」；大小写按随包写法判，闸门之前学进去的小写 sm 也管得住。冻结日志回放与上一提交逐条相同。"
```

---

### Task 4: (c1) 上屏后整个删掉，撤销那次英文学习

**Files:**
- Create: `crates/qingjian-core/src/engine/learning/english/retract.rs`
- Modify: `crates/qingjian-core/src/engine/learning/english/mod.rs`（`mod retract;`）
- Modify: `crates/qingjian-core/src/engine/learning/learner.rs:56`（`learn_english` 之后）
- Modify: `crates/qingjian-core/src/engine/learning/muted.rs:123`（`learn_english` 之后）
- Modify: `crates/qingjian-core/src/engine/commit/last.rs:35-52`
- Modify: `crates/qingjian-core/src/engine/prediction/mod.rs:314-325`
- Modify: `crates/qingjian-core/src/engine/commit/mod.rs`（`learned_english`、两处 `LastCommit`）
- Modify: `crates/qingjian-core/src/engine/composing.rs:139-154`（`note_backspace`）、`take_raw` 的 `remember_commit`
- Modify: `crates/qingjian-learning/src/frequency_learner/learner_impl.rs`、`tests.rs`
- Modify: `cloud/crates/qingjian-cloud-bridge/src/scope/scoped_learner.rs`
- Test: `crates/qingjian-core/src/engine/tests/english_learning.rs`
- Docs: `docs/user/input/english-mode.md`、`docs/notes/crate-notes.md`、`cloud/docs/fork-patch.md`

- [ ] **Step 1: 写失败的测试**

`english_learning.rs` 的 `impl Learner for SpyLearner` 里 `learn_english` 之后加：

```rust
    fn unlearn_english(&mut self, word: &str) {
        self.log.lock().unwrap().push(format!("unlearn:{word}"));
        if let Some((_, n)) = self
            .english
            .iter_mut()
            .find(|(w, _)| w.eq_ignore_ascii_case(word))
        {
            *n -= 1;
        }
        self.english.retain(|(_, n)| *n > 0);
        self.rebuild();
    }
```

文件末尾追加：

```rust
#[test]
fn erasing_a_raw_english_word_unlearns_it() {
    let (mut engine, log) = spy_engine();
    raw(&mut engine, "gist");
    for _ in 0..4 {
        engine.note_backspace();
    }
    assert_eq!(learned(&log), ["learn:gist", "unlearn:gist"]);
    assert!(personal(&engine).is_empty());
}

#[test]
fn erasing_part_of_a_raw_english_word_keeps_it() {
    let (mut engine, log) = spy_engine();
    raw(&mut engine, "gist");
    engine.note_backspace();
    assert_eq!(learned(&log), ["learn:gist"]);
    assert_eq!(personal(&engine), ["gist"]);
}

#[test]
fn erasing_a_picked_english_candidate_also_takes_back_the_pick() {
    let (mut engine, log) = spy_engine();
    pick(&mut engine, "ok", "OK");
    assert_eq!(engine.learner().weight("OK"), 1);
    engine.note_backspace();
    engine.note_backspace();
    assert_eq!(learned(&log), ["learn:OK", "unlearn:OK"]);
    assert_eq!(engine.learner().weight("OK"), 0);
}
```

`crates/qingjian-learning/src/frequency_learner/tests.rs` 末尾追加：

```rust
#[test]
fn unlearning_english_counts_down_and_drops_at_zero() {
    let mut learner = FrequencyLearner::default();
    learner.learn_english("gist");
    learner.learn_english("gist");
    learner.unlearn_english("Gist");
    assert_eq!(learner.english_count(), 1);
    learner.unlearn_english("gist");
    assert_eq!(learner.english_count(), 0);
    assert!(learner.user_english().is_none());
    learner.unlearn_english("gist");
    assert_eq!(learner.english_count(), 0, "没有的词撤销不出错");
}
```

Run: `cargo test -p qingjian-core english_learning`
Expected: 编译失败（trait 里没有 `unlearn_english`）。

- [ ] **Step 2: trait、包装层、存储**

`learning/learner.rs`，`fn learn_english(&mut self, _word: &str) {}` 之后：

```rust

    /// 撤销一次 [`Self::learn_english`]：用户上屏后马上整个删掉了。次数减到 0 就从个人英文词表删掉。
    fn unlearn_english(&mut self, _word: &str) {}
```

`learning/muted.rs`，`fn learn_english` 实现之后：

```rust
    // 与 learn_english 对称：没记的不撤
    fn unlearn_english(&mut self, word: &str) {
        if !self.muted() && !mnemonic::is_bip39_word(word) {
            self.inner.unlearn_english(word);
        }
    }
```

`crates/qingjian-learning/src/frequency_learner/learner_impl.rs`，`fn learn_english` 之后：

```rust
    fn unlearn_english(&mut self, word: &str) {
        let code = word.to_ascii_lowercase();
        let Some(entry) = self.english.get_mut(&code) else {
            return;
        };
        entry.1 = entry.1.saturating_sub(1);
        if entry.1 == 0 {
            self.english.remove(&code);
        }
        self.english_dirty = true;
        self.rebuild_english();
        tracing::debug!(word, "撤销个人英文词");
    }
```

`cloud/crates/qingjian-cloud-bridge/src/scope/scoped_learner.rs`，`fn user_english` 之前：

```rust
    // 与 learn_english 同一层：选了人退对象层，没选人退全局
    fn unlearn_english(&mut self, word: &str) {
        if !self.write_contact_words(|layer| layer.unlearn_english(word)) {
            self.global.unlearn_english(word);
        }
    }
```

- [ ] **Step 3: `LastCommit` 记这次学了哪个英文词**

`commit/last.rs`，结构体 `pub phrase_learned: bool,` 之后：

```rust

    /// 这次上屏学进个人英文词表的词（回车原样上屏、英文候选、句末英文词）：整个删掉就退回那次学习。
    pub english: Option<String>,

    /// 上面那个英文词同时记过一次选择次数（`Learner::record`，选英文候选时才有），退回时一起退。
    pub english_recorded: bool,
```

`LastCommit::plain` 与 `prediction/mod.rs` 里的 `LastCommit { … phrase_learned: false, }` 字面量各补：

```rust
            english: None,
            english_recorded: false,
```

`commit/mod.rs`：`let mut typos = Vec::new();` 之后加 `let mut learned_english = None;`；英文分支里 `self.note_english_commit(&candidate.text);` 之后加

```rust
                        learned_english = Some(candidate.text.clone());
```

函数末尾构造 `commit` 的两支：

```rust
// 改前
                phrase,
                phrase_learned,
            }
        } else {
            let mut plain = LastCommit::plain(&candidate.text);
            plain.chars = traditional_text.chars().count();
            plain
        };
// 改后
                phrase,
                phrase_learned,
                english: sentence_english,
                english_recorded: false,
            }
        } else {
            let mut plain = LastCommit::plain(&candidate.text);
            plain.chars = traditional_text.chars().count();
            plain.english_recorded = learned_english.is_some();
            plain.english = learned_english;
            plain
        };
```

`composing.rs` 的 `take_raw`：

```rust
// 改前
        self.remember_commit(LastCommit::plain(&raw));
// 改后
        let mut commit = LastCommit::plain(&raw);
        commit.english = english_word.then(|| raw.clone());
        self.remember_commit(commit);
```

- [ ] **Step 4: 退格删净时撤销**

`learning/english/retract.rs`（新文件）：

```rust
//! 上屏后整个删掉：那次学进个人英文词表的词退回去（WeType「上屏后第一次文本变化是删除就撤销」，这里以删干净为准）。

use crate::engine::Engine;
use crate::engine::learning::Learner;

impl Engine {
    /// 退回一次英文学习；`recorded` 是那次上屏同时记过选择次数（选的英文候选）。
    pub(in crate::engine) fn retract_english(&mut self, word: &str, recorded: bool) {
        self.learner.unlearn_english(word);
        if recorded {
            self.learner.unrecord(word);
        }
        self.forget_span_cache();
        tracing::debug!(word, "上屏的英文词删掉了，撤销那次学习");
    }
}
```

`learning/english/mod.rs`：`mod origin;` 之后加 `mod retract;`（在 `mod sanitize;` 之前）。

`composing.rs` 的 `note_backspace` 末尾：

```rust
// 改前
        commit.erased += 1;
        if commit.is_erased() {
            // 刚上屏的词没了，它不再是下一个词的上文
            self.chain.reset();
        }
    }
// 改后
        commit.erased += 1;
        if !commit.is_erased() {
            return;
        }
        let english = commit
            .english
            .take()
            .map(|word| (word, commit.english_recorded));
        // 刚上屏的词没了，它不再是下一个词的上文
        self.chain.reset();
        if let Some((word, recorded)) = english {
            self.retract_english(&word, recorded);
        }
    }
```

Run:
```bash
cargo fmt --all && cargo test -p qingjian-core -p qingjian-learning
cd cloud && cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test -p qingjian-cloud-bridge && cd ..
```
Expected: core `377 passed`、learning 全过、云桥全过。

- [ ] **Step 5: 回放**

同 Task 1 Step 8（`/tmp/d3-c1.jsonl`）。Expected：与 Task 3 逐条相同（冻结日志里 34 次退格撤销都是中文词）。

- [ ] **Step 6: 文档**

`docs/user/input/english-mode.md`「## 记住用户的词」加一句：

```markdown
英文词上屏后马上整个删掉，这次就不记。
```

`docs/notes/crate-notes.md`「个人英文词的学习闸门」那条末尾追加：

```markdown
  上屏后退格删干净（`note_backspace`）时按 `LastCommit.english` 退回那次学习（`Learner::unlearn_english`，次数减到 0 删词；选的英文候选同时 `unrecord`）。
```

`cloud/docs/fork-patch.md`「英文学习闸门」表里追加：

```markdown
| `crates/qingjian-core/src/engine/learning/{learner,muted}.rs` | 加 1 个缺省方法 + 转发 | `unlearn_english`；`MutedLearner` 与 `learn_english` 对称地挡 |
| `crates/qingjian-core/src/engine/commit/last.rs`、`prediction/mod.rs` | 加 2 个字段 | `LastCommit.english` / `english_recorded` |
| `crates/qingjian-core/src/engine/composing.rs` | 改约 12 行 | `note_backspace` 删净时调 `retract_english` |
| `crates/qingjian-learning/src/frequency_learner/learner_impl.rs` | 加约 14 行 | `unlearn_english` |
```

- [ ] **Step 7: 提交**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test -p qingjian-core -p qingjian-learning
git add crates cloud/crates/qingjian-cloud-bridge docs/user/input/english-mode.md docs/notes/crate-notes.md cloud/docs/fork-patch.md
git commit -m "fix(core): 英文词上屏后整个删掉就撤销那次学习" -m "退格撤销（apply_retraction）只退中文的选择与转移，误按空格上屏的英文照样记进个人词表。
照 WeType 的撤销语义，以「整个删掉」为准（只退一个字母不算）；选的英文候选连选择次数一起退。"
```

---

### Task 5: (c2) 个人英文词衰减：用过不到 5 次、30 天没用就删

**Files:**
- Create: `crates/qingjian-learning/src/retention/mod.rs`、`crates/qingjian-learning/src/retention/policy.rs`
- Create: `crates/qingjian-learning/src/frequency_learner/sweep.rs`
- Modify: `crates/qingjian-learning/src/lib.rs`
- Modify: `crates/qingjian-learning/src/frequency_learner/{mod,tables,learner_impl,tests}.rs`
- Modify: `cloud/crates/qingjian-cloud-client/src/learning/{snapshot,tests}.rs`
- Docs: `docs/user/input/english-mode.md`、`docs/user/help/data-and-logs.md`、`docs/notes/crate-notes.md`、`cloud/docs/fork-patch.md`

- [ ] **Step 1: 写判据与它的测试**

`crates/qingjian-learning/src/retention/mod.rs`（新文件）：

```rust
//! 学习数据的保留判据：用得少又闲置太久的条目删掉。现在只用在个人英文词表上。
//! 判据照微信输入法的 StaticTarget（见 `docs/notes/wetype-impl/c1-c3-cleanup-offline-space.md` C1 节）。

mod policy;

pub use policy::RetentionPolicy;

use std::time::{SystemTime, UNIX_EPOCH};

/// 今天是 Unix 纪元以来第几天（UTC），学习数据里「最后使用日」的单位。
pub fn today() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| (elapsed.as_secs() / 86_400) as u32)
}
```

`crates/qingjian-learning/src/retention/policy.rs`（新文件）：

```rust
//! 固定阈值的保留判据：次数不到下限且闲置超过天数就删，摊到每次学词时看后面几条。

/// 保留判据。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetentionPolicy {
    /// 用过这么多次的不删。
    pub min_count: u32,

    /// 闲置超过这么多天（不含）才删。
    pub max_idle_days: u32,

    /// 每学一条顺带检查后面几条。
    pub check_next: usize,
}

impl RetentionPolicy {
    /// 个人英文词：用过不到 5 次、30 天没用过就删，每学一个词看后面 10 条（微信输入法英文库的 StaticTarget 缺省值）。
    pub const STATIC_ENGLISH: Self = Self {
        min_count: 5,
        max_idle_days: 30,
        check_next: 10,
    };

    /// 这一条该不该删。`last_day` 与 `today` 都是 Unix 天数。
    pub fn should_drop(&self, count: u32, last_day: u32, today: u32) -> bool {
        count < self.min_count && today.saturating_sub(last_day) > self.max_idle_days
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_english_drops_rare_and_idle_entries_only() {
        let policy = RetentionPolicy::STATIC_ENGLISH;
        assert!(policy.should_drop(4, 969, 1000), "闲置 31 天、4 次：删");
        assert!(!policy.should_drop(4, 970, 1000), "闲置正好 30 天：留");
        assert!(!policy.should_drop(5, 0, 1000), "次数够：留");
        assert!(!policy.should_drop(1, 1001, 1000), "时钟回拨：留");
    }
}
```

`crates/qingjian-learning/src/lib.rs`：

```rust
// 改前
mod input_log;
mod usage_stats;
…
pub use input_log::InputLog;
// 改后
mod input_log;
mod retention;
mod usage_stats;
…
pub use input_log::InputLog;
pub use retention::RetentionPolicy;
```

Run: `cargo test -p qingjian-learning retention`
Expected: 1 passed。

- [ ] **Step 2: 写衰减的失败测试**

`crates/qingjian-learning/src/frequency_learner/tests.rs` 末尾追加：

```rust
#[test]
fn rare_idle_english_words_decay_while_learning() {
    let mut learner = FrequencyLearner::default();
    learner.load_english(
        "# 青简个人英文词 v2：词\\t次数\\t最后使用日\nial\t1\t100\nsm\t4\t969\ngist\t5\t0\nrust\t1\t990\n",
    );
    assert_eq!(learner.sweep_english(1000), 2, "ial（4 次以下、闲置 900 天）与 sm（闲置 31 天）删");
    assert_eq!(learner.english_count(), 2, "gist 次数够、rust 闲置 10 天：留");
    assert!(learner.has_unsaved());
}

#[test]
fn english_decay_checks_ten_entries_per_step_and_wraps() {
    let mut learner = FrequencyLearner::default();
    let rows: String = (0..12).map(|i| format!("w{i:02}\t1\t0\n")).collect();
    learner.load_english(&format!("# 青简个人英文词 v2：词\\t次数\\t最后使用日\n{rows}"));
    assert_eq!(learner.sweep_english(1000), 10);
    assert_eq!(learner.english_count(), 2);
    assert_eq!(learner.sweep_english(1000), 2, "接着上次的位置，到尾绕回开头");
    assert_eq!(learner.english_count(), 0);
}

#[test]
fn english_last_used_day_round_trips() {
    let dir = std::env::temp_dir().join(format!("qingjian-english-days-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("user.tsv");
    std::fs::write(
        dir.join(USER_ENGLISH_FILE),
        "# 青简个人英文词 v2：词\\t次数\\t最后使用日\ngist\t5\t123\n",
    )
    .unwrap();
    let mut learner = FrequencyLearner::from_path(&path).unwrap();
    learner.learn_english("python");
    learner.flush();
    let saved = std::fs::read_to_string(dir.join(USER_ENGLISH_FILE)).unwrap();
    assert!(saved.contains("gist\t5\t123\n"), "{saved}");
    assert!(saved.contains(&format!("python\t1\t{}\n", crate::retention::today())));
    std::fs::remove_dir_all(dir).unwrap();
}
```

Run: `cargo test -p qingjian-learning`
Expected: 编译失败（`sweep_english` 不存在）。

- [ ] **Step 3: 实现衰减与第三列**

`crates/qingjian-learning/src/frequency_learner/sweep.rs`（新文件）：

```rust
//! 个人英文词的衰减：每学一个英文词，从上次停下的地方往后看几条，用得少又闲置太久的删掉（[`RetentionPolicy::STATIC_ENGLISH`]）。

use std::ops::Bound;

use super::FrequencyLearner;
use crate::retention::RetentionPolicy;

impl FrequencyLearner {
    /// 从游标起检查 [`RetentionPolicy::check_next`] 条（到尾绕回开头），删掉该删的，返回删了几条。
    /// 没有「最后使用日」的条目（同步合并进来的）当作今天刚用过。
    pub(super) fn sweep_english(&mut self, today: u32) -> usize {
        let policy = RetentionPolicy::STATIC_ENGLISH;
        let cursor = self.english_cursor.take().unwrap_or_default();
        let after = self
            .english
            .range::<str, _>((Bound::Excluded(cursor.as_str()), Bound::Unbounded));
        let before = self
            .english
            .range::<str, _>((Bound::Unbounded, Bound::Included(cursor.as_str())));
        let checked: Vec<(String, u32)> = after
            .chain(before)
            .take(policy.check_next)
            .map(|(code, (_, count))| (code.clone(), *count))
            .collect();
        self.english_cursor = checked.last().map(|(code, _)| code.clone());
        let mut dropped = 0;
        for (code, count) in checked {
            let last_day = *self.english_days.entry(code.clone()).or_insert(today);
            if policy.should_drop(count, last_day, today) {
                self.english.remove(&code);
                self.english_days.remove(&code);
                dropped += 1;
            }
        }
        if dropped > 0 {
            self.english_dirty = true;
            self.rebuild_english();
            tracing::debug!(dropped, "个人英文词衰减");
        }
        dropped
    }
}
```

`frequency_learner/mod.rs`：`mod remote;` 之后加 `mod sweep;`；结构体 `english_legacy: bool,` 之后加：

```rust

    /// 个人英文词的最后使用日（小写编码 → Unix 天数），衰减用；没有的当今天。
    english_days: BTreeMap<String, u32>,

    /// 衰减上次检查到的编码，下次接着往后看。
    english_cursor: Option<String>,
```

`frequency_learner/tables.rs`：导入加 `use crate::retention;`（在 `use crate::error::LearningError;` 之后）。`load_english` 换成：

```rust
    /// 从 `词\t次数\t最后使用日` 读个人英文词，返回跳过的坏行数。没有最后使用日的（旧文件）当今天；再往后多出的列不管。
    pub(super) fn load_english(&mut self, source: &str) -> usize {
        let today = retention::today();
        let mut skipped = 0;
        for line in data_lines(source) {
            let mut columns = line.split('\t');
            let Some((word, count)) = columns
                .next()
                .zip(columns.next())
                .and_then(|(word, count)| Some((word, parse_count(count)?)))
            else {
                skipped += 1;
                continue;
            };
            if count > 0 {
                let code = word.to_ascii_lowercase();
                // 旧文件没有最后使用日：当作今天，从升级那天起算闲置，不会一升级就删
                let day = columns
                    .next()
                    .and_then(|day| day.parse().ok())
                    .unwrap_or(today);
                self.english_days.insert(code.clone(), day);
                self.english.insert(code, (word.to_owned(), count));
            }
        }
        self.english_legacy =
            !self.english.is_empty() && !source.trim_start().starts_with(USER_ENGLISH_VERSION);
        self.rebuild_english();
        skipped
    }
```

`save_english_to` 里排序与写出：

```rust
// 改前
        let mut rows: Vec<&(String, u32)> = self.english.values().collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
        write_atomic(path, |file| {
            writeln!(file, "{USER_ENGLISH_VERSION}：词\\t次数")?;
            for (word, count) in rows {
                writeln!(file, "{word}\t{count}")?;
            }
// 改后
        let today = retention::today();
        let mut rows: Vec<(&str, u32, u32)> = self
            .english
            .iter()
            .map(|(code, (word, count))| {
                let day = self.english_days.get(code).copied().unwrap_or(today);
                (word.as_str(), *count, day)
            })
            .collect();
        rows.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
        write_atomic(path, |file| {
            writeln!(file, "{USER_ENGLISH_VERSION}：词\\t次数\\t最后使用日")?;
            for (word, count, day) in rows {
                writeln!(file, "{word}\t{count}\t{day}")?;
            }
```

`frequency_learner/learner_impl.rs`：导入 `use super::{FrequencyLearner, MAX_CHOICE_ENTRIES, RAW_MARK, bump};` 之后加 `use crate::retention;`；`learn_english` 里：

```rust
// 改前
        bump(&mut entry.1);
        let count = entry.1;
        self.english_dirty = true;
        self.rebuild_english();
// 改后
        bump(&mut entry.1);
        let count = entry.1;
        let today = retention::today();
        self.english_days.insert(word.to_ascii_lowercase(), today);
        self.english_dirty = true;
        self.sweep_english(today);
        self.rebuild_english();
```

Run: `cargo fmt --all && cargo test -p qingjian-learning`
Expected: 全过（含 Task 2 的 `extra_english_columns_are_ignored`，第三列现在读成最后使用日，次数仍是 1 条）。

- [ ] **Step 4: 云同步客户端不把第三列当次数（差异 10，不改会让同步把英文词全推成删除）**

`cloud/crates/qingjian-cloud-client/src/learning/tests.rs` 末尾追加：

```rust

/// 输入法给个人英文词表加了「最后使用日」第三列：同步只认词与次数，第三列不当次数读。
#[test]
fn english_last_used_day_column_is_not_a_count() {
    let mut old = Snapshot::default();
    old.parse_table(Table::English, "# 青简个人英文词：词\\t次数\ngist\t2\n");
    let mut new = Snapshot::default();
    new.parse_table(
        Table::English,
        "# 青简个人英文词 v2：词\\t次数\\t最后使用日\ngist\t2\t20737\n",
    );
    assert!(!new.is_empty());
    assert_eq!(new, old);
}
```

Run: `cd cloud && cargo test -p qingjian-cloud-client english_last_used_day && cd ..`
Expected: FAILED（`new` 是空的：`gist\t2` 被当成两列键、键列数不对跳过）。

`cloud/crates/qingjian-cloud-client/src/learning/snapshot.rs` 的 `parse_table`：

```rust
// 改前
            let fields: Vec<&str> = line.split('\t').collect();
            if table.is_set() {
// 改后
            let mut fields: Vec<&str> = line.split('\t').collect();
            // 个人英文词表第三列是最后使用日（输入法衰减用），不同步
            if table == Table::English && fields.len() == 3 {
                fields.pop();
            }
            if table.is_set() {
```

Run: `cd cloud && cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test && cd ..`
Expected: 全过。

- [ ] **Step 5: 文档**

`docs/user/input/english-mode.md`「## 记住用户的词」加一句：

```markdown
用过不到 5 次、30 天没再用的英文词会自动从个人词表里移除。
```

`docs/user/help/data-and-logs.md`：

```markdown
| `user-english.tsv` | 原样上屏过、选过的英文词，及最后一次使用的日期 | 英文词 |
```

`docs/notes/crate-notes.md` qingjian-learning 段，`个人英文词（\`user-english.tsv\`，` 这一处改成 `个人英文词（\`user-english.tsv\`，\`词\t次数\t最后使用日\`（Unix 天数），版本头 \`# 青简个人英文词 v2\`，`；
并在该条「越打不出来学得越多）、」之后插一行：

```markdown
  个人英文词衰减（`frequency_learner/sweep.rs`、`retention::RetentionPolicy::STATIC_ENGLISH`）：每学一个英文词从游标起看 10 条，用过不到 5 次且闲置超过 30 天的删，
  照 WeType 英文库的 StaticTarget；旧文件没有最后使用日按加载当天算。云同步客户端读这张表时丢掉第三列（`cloud-client` `Snapshot::parse_table`）。
```

`cloud/docs/fork-patch.md`「英文学习闸门」表里追加：

```markdown
| `crates/qingjian-learning/src/retention/`、`frequency_learner/sweep.rs` | 新文件 | `RetentionPolicy::STATIC_ENGLISH`（5 次 / 30 天 / 每步 10 条）、英文词衰减 |
| `crates/qingjian-learning/src/{lib.rs,frequency_learner/{mod,tables,learner_impl}.rs}` | 加约 30 行 | `user-english.tsv` 第三列最后使用日；`learn_english` 后顺带衰减 |
| `cloud/crates/qingjian-cloud-client/src/learning/snapshot.rs` | 加 3 行 | 英文表第三列不同步（不改会把最后使用日当次数、整行跳过） |
```

- [ ] **Step 6: 提交**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test -p qingjian-learning
git add crates/qingjian-learning cloud/crates/qingjian-cloud-client docs/user docs/notes/crate-notes.md cloud/docs/fork-patch.md
git commit -m "feat(learning): 个人英文词用过不到 5 次、30 天没用就删" -m "learn_english 只加不减（todo 2026-09-24），打不出来时攒下的碎片永久占位。
照 WeType 英文库的 StaticTarget：每学一个词从游标起看 10 条，不做全表扫，不在按键路径上多花时间。
user-english.tsv 加第三列最后使用日，旧文件按加载当天算，不会一升级就删；云同步客户端同步只认前两列。"
```

---

### Task 6: (c3) 中文在这段字母下多选 3 次以上，英文让到第 5 位

**Files:**
- Modify: `crates/qingjian-core/src/engine/learning/english/mod.rs`（加常数）
- Modify: `crates/qingjian-core/src/engine/learning/mod.rs`（re-export）
- Modify: `crates/qingjian-core/src/engine/extras.rs`（`insert_english` 的 `position`）
- Test: `crates/qingjian-core/src/engine/tests/english_learning.rs`
- Docs: `docs/user/input/english-mode.md`、`docs/notes/crate-notes.md`、`docs/plan/todo.md`、`docs/plan/roadmap.md`、`cloud/docs/fork-patch.md`

- [ ] **Step 1: 写失败的测试**

`english_learning.rs` 末尾追加：

```rust
/// `key` 下的候选不少于五个：精确英文词 key 让位时落在第 5 位。
const KEY_DICT: &str = "可以\tke yi\t9000\n客运\tke yun\t800\n刻意\tke yi\t700\n科研\tke yan\t600\n课业\tke ye\t500\n可疑\tke yi\t400\n";

fn key_engine() -> Engine {
    Engine::new(Dictionary::parse(KEY_DICT).unwrap())
        .with_english(WordList::parse(WORDS).unwrap())
        .with_learner(Box::new(SpyLearner::default()))
}

#[test]
fn english_keeps_second_place_until_chinese_leads_by_the_margin() {
    let mut engine = key_engine();
    pick(&mut engine, "key", "key");
    for _ in 0..3 {
        pick(&mut engine, "key", "可以");
    }
    engine.set_input("key");
    assert_eq!(texts_of(&engine)[1], "key", "多 2 次：还在第 2");
    pick(&mut engine, "key", "可以");
    engine.set_input("key");
    let all = texts_of(&engine);
    assert_eq!(all[0], "可以");
    assert_eq!(all[4], "key", "多 3 次：让到第 5，{all:?}");
}

#[test]
fn demoted_english_goes_last_when_the_page_is_short() {
    let dictionary = Dictionary::parse("可以\tke yi\t9000\n客运\tke yun\t100\n").unwrap();
    let mut engine = Engine::new(dictionary)
        .with_english(WordList::parse(WORDS).unwrap())
        .with_learner(Box::new(SpyLearner::default()));
    for _ in 0..3 {
        pick(&mut engine, "key", "可以");
    }
    engine.set_input("key");
    let all = texts_of(&engine);
    assert_eq!(all.last().map(String::as_str), Some("key"), "{all:?}");
}
```

Run: `cargo test -p qingjian-core english_learning`
Expected: 这两条 FAILED（key 一直在第 2）。

- [ ] **Step 2: 实现**

`learning/english/mod.rs` 末尾加：

```rust

/// 同一段字母下中文首选比这个英文词多选过这么多次，英文从第 2 位让到第一页第 5 位（todo「个人英文词表没有衰减」）。
pub(in crate::engine) const ENGLISH_DEMOTE_MARGIN: u32 = 3;
```

`learning/mod.rs`：

```rust
// 改前
pub(super) use english::{EnglishOrigin, SHORT_ENGLISH_FIRST_PICKS};
// 改后
pub(super) use english::{ENGLISH_DEMOTE_MARGIN, EnglishOrigin, SHORT_ENGLISH_FIRST_PICKS};
```

`extras.rs` 的 `insert_english`：

```rust
// 改前
        let mut position = if items.is_empty() || english_first {
            0
        } else {
            1
        };
// 改后
        // 分叉补丁：用户在这段字母下一直选中文（选得比英文多 ENGLISH_DEMOTE_MARGIN 次以上），英文让到第一页末尾
        let demoted = !english_first
            && chosen >= english_weight.saturating_add(learning::ENGLISH_DEMOTE_MARGIN);
        let mut position = if items.is_empty() || english_first {
            0
        } else if demoted {
            items.len().min(4)
        } else {
            1
        };
```

Run: `cargo fmt --all && cargo test -p qingjian-core`
Expected: `test result: ok. 379 passed`（`english.rs` 的 `english_word_yields_to_a_chinese_word_the_user_keeps_choosing` 只选一次中文，不受影响）。

- [ ] **Step 3: 回放**

同 Task 1 Step 8（`/tmp/d3-c3.jsonl`，比 `/tmp/d3-before.jsonl`）。
Expected：词 920/1057、整句 102/165、英文 15/21 不变；逐条差异多出一批「随包英文词从第 2 位退到第 5 位」的行（`de` → DE、`ni` → Ni、`ba` → BA、`d` → D、`he` → he、`yong` → Yong…，
所选词名次都不变），唯一名次变化是第 2239 行 `ba` → 吧 从第 3 名升到第 2 名，加上 Task 1 的第 1869 行 `ac`。整句评测不变。

- [ ] **Step 4: 文档**

`docs/user/input/english-mode.md`「## 记住用户的词」里已有这段：

```markdown
中文模式下 `key`、`id` 这类末尾落单一个字母、既像英文词又可读成拼音的输入，若常选中文词（如「客一」「以的」），中文词排在前面，英文词退后；
勾选「中文候选排在英文词前面」后中文词一律在前。
```

改成：

```markdown
中文模式下 `key`、`id` 这类末尾落单一个字母、既像英文词又可读成拼音的输入，若常选中文词（如「客一」「以的」），中文词排在前面，英文词退后；
选中文比选这个英文词多 3 次以上，英文词退到第一页末尾（第 5 位）。勾选「中文候选排在英文词前面」后中文词一律在前。
```

`docs/notes/crate-notes.md`「- 中英混输的英文词位置：…」那条第二行 `用户老选中文词时仍让中文在前），` 改成
`用户老选中文词时仍让中文在前，同一段字母下中文首选比英文多选 \`ENGLISH_DEMOTE_MARGIN\`（3）次以上英文让到第 5 位），`。

`docs/plan/todo.md`：删掉「- [ ] 个人英文词表没有衰减（2026-09-24 发现）：…现成出口：删候选快捷键（缺省 `Shift` + 数字）走的就是 `forget_english`。」整条（5 行）。
`docs/plan/roadmap.md` Phase 4 里「- [ ] 可选复习功能」之前加：

```markdown
- [x] 个人英文词学习闸门（2026-10-11，分叉补丁，上游 #545）：残段与短串回车不学、两字母英文选两次才排第一、退格删净撤销、
  用过不到 5 次且 30 天没用自动删、中文多选 3 次英文让到第 5 位；旧词表与云同步合并进来的词也过闸门（`cloud/docs/fork-patch.md`）
```

`cloud/docs/fork-patch.md`「英文学习闸门」表里追加：

```markdown
| `crates/qingjian-core/src/engine/extras.rs` | 加约 6 行 | 中文首选比英文多选 3 次以上，英文从第 2 位让到第 5 位 |
```

- [ ] **Step 5: 提交**

```bash
cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test -p qingjian-core
git add crates/qingjian-core docs/user/input/english-mode.md docs/notes/crate-notes.md docs/plan/todo.md docs/plan/roadmap.md cloud/docs/fork-patch.md
git commit -m "fix(core): 一直选中文的字母串，英文让到第一页第 5 位" -m "原来英文的位置只有 0 和 1 两档，用户选一百次中文，那条英文还钉在第二位（todo 2026-09-24）。
中文首选比英文多选 3 次以上让到第 5 位（候选不足 5 个时在末尾）。冻结日志：命中数不变，ba → 吧 从第 3 升到第 2；
本机日志：词平均名次 1.59 → 1.57，英文平均名次 1.67 → 2.17（6 条，首选与前五不变）。"
```

（本机日志的数字以 Task 7 Step 2 实际跑出来的为准，提交前改成你的数。）

---

### Task 7: 全量验证、回放数字入档

**Files:**
- Modify: `cloud/docs/fork-patch.md`（「英文学习闸门」一节末尾加回放段落）

- [ ] **Step 1: 全量测试与 lint**

```bash
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test
cd cloud && cargo fmt --all --check && cargo clippy --all-targets -- -D warnings && cargo test && cd ..
```
Expected：全部通过，无 FAILED。

- [ ] **Step 2: 回放与评测（改后 vs Task 0）**

```bash
cargo build --release -p qingjian-cli
CLI=./target/release/qingjian-cli
$CLI --config /tmp/empty.toml --replay data/eval/input-log-2026-10-04.jsonl --misses 0 2>/dev/null | grep -E "^(词|整句|英文)"
$CLI --config /tmp/empty.toml --eval-text data/eval/sentences.tsv --misses 0 2>/dev/null | grep -E "^句子"
$CLI --config tools/eval/offline.toml --replay data/eval/input-log-2026-10-04.jsonl --clean2 \
  --extra-dict data/generated/dicts/idioms.qj --misses 0 2>/dev/null | sed -n '/干净口径 2/,/整句 /p' | grep -E "词|整句" | diff /tmp/d3-clean2-before.txt -
$CLI --config tools/eval/offline.toml --eval-text data/eval/sentences.tsv \
  --extra-dict data/generated/dicts/idioms.qj --misses 0 2>/dev/null | grep -E "^句子" | diff /tmp/d3-eval-before.txt -
$CLI --config /tmp/empty.toml --replay "$HOME/Library/Application Support/Qingjian/input-log.jsonl" --misses 0 2>/dev/null \
  | grep -E "^(词|整句|英文)" | diff /tmp/d3-local-before.txt -
```

判据：
- 冻结日志：词 **920/1057 不变**、整句 102/165 不变、英文 **16/21 → 15/21**（第 1869 行 `ac`）。
- 整句评测（空配置与 offline.toml 两种）、clean2 回放：**与 Task 0 逐字相同**（两个 `diff` 无输出）。
- 本机日志：词与整句命中数不变；只允许平均名次变化（2026-10-11 实测词 1.59 → 1.57、英文 1.67 → 2.17）。
- 有通变模型（`data/models/hanzhang-tongbian/hanzhang-tongbian-small.qjm`）的话，上面两条 offline.toml 命令各加 `--neural <模型>` 再比一遍，同样逐字相同。

- [ ] **Step 3: 可选的本机冒烟（不要求，Core 有单测与回放兜底）**

`apps/macos/scripts/bundle.sh --install` 后在 TextEdit 里：`wolai` 选「我啦」再按空格选 I；`sm` 回车；再打 `sm` 看首选是不是「什么」；
`~/Library/Application Support/Qingjian/user-english.tsv` 第一行应是 `# 青简个人英文词 v2：词\t次数\t最后使用日`，里面没有 `i` / `sm`。

- [ ] **Step 4: 回放数字入档**

`cloud/docs/fork-patch.md`「英文学习闸门」一节表格之后加（数字换成 Step 2 实测）：

```markdown
回放（`qingjian-cli --config /tmp/empty.toml --replay`）：冻结日志 `input-log-2026-10-04.jsonl` 词首选 920/1057、整句 102/165 不变，
英文 16/21 → 15/21——少的是第 1869 行当时回车学进去的小写 `ac`（现在不学，`ac` 首选变 啊吃、随包 AC 第 3，回车照样打得出 ac）；
让位规则把 `de` → DE、`ni` → Ni 这类随包英文词从第 2 退到第 5，`ba` → 吧 从第 3 升到第 2。本机日志词平均名次 1.59 → 1.57，命中数不变。
整句评测（1878 句）与 clean2 回放改前改后逐字相同。

合并上游时：冲突面在 `composing.rs` 的 `take_raw` / `note_backspace`、`commit/mod.rs` 的英文分支、`extras.rs::insert_english`、`setup.rs` 三个构造挂钩；
上游若改了 `user-english.tsv` 的格式，`tables.rs` 的版本头与第三列、`cloud-client` 的 `parse_table` 要一起看。
```

- [ ] **Step 5: 提交**

```bash
git add cloud/docs/fork-patch.md
git commit -m "docs(cloud): 英文学习闸门的回放数字与合并上游时的冲突面"
```

---

### Task 8: 推分支、开 PR（不合并）

- [ ] **Step 1: 推送到自己的仓库**

```bash
git remote -v   # 确认 origin 是 dayuer/qingjian
git push -u origin english-gate/d3
```

（`.githooks/pre-push` 会跑全 workspace 测试。）

- [ ] **Step 2: 开 PR**

```bash
gh pr create --repo dayuer/qingjian --base sujian --head english-gate/d3 \
  --title "fix(core): 英文学习闸门——残段 / 短串不学、退格撤销、衰减与让位（上游 #545）" \
  --body "$(cat <<'EOF'
## 做了什么

修上游 #545 的两条复现路径（`wolai` 选「我啦」剩下的 `i` 选 I 就学；`sm` 回车一次就压过「什么」），并补 todo「个人英文词表没有衰减」。六个提交可以分别回滚：

1. (a) 残段不学；中文模式回车串不到 3 个字母不学、不到 5 个字母又不在随包英文表的不学
2. (d) 旧 `user-english.tsv`（没有版本头）在随包英文表挂上时按新规则清一次；云同步合并进来的英文词也过闸门
3. (b′) 两个字母的英文词要选两次才排到中文前面
4. (c1) 英文词上屏后整个删掉，撤销那次学习（连选择次数）
5. (c2) 用过不到 5 次、30 天没用的个人英文词删掉（`user-english.tsv` 加第三列最后使用日，云同步客户端只认前两列）
6. (c3) 同一段字母下中文比英文多选 3 次以上，英文让到第 5 位

规格 `docs/notes/wetype-impl/d3-learning-gate.md`，计划 `cloud/docs/plans/2026-10-11-english-learning-gate.md`，分叉补丁记在 `cloud/docs/fork-patch.md`「英文学习闸门」。

## 怎么验证的

- `cargo test`（workspace）与 `cd cloud && cargo test` 全过；两边 `cargo clippy --all-targets -- -D warnings` 干净。
  新测试 `crates/qingjian-core/src/engine/tests/english_learning.rs` 逐条对应规格 §6 单测清单（含 #545 两条复现路径）。
- 冻结日志回放 `qingjian-cli --config /tmp/empty.toml --replay data/eval/input-log-2026-10-04.jsonl`：词首选 920/1057 不变，英文 16/21 → 15/21（第 1869 行 `ac`，当时回车学进去的小写 ac）。
- 整句评测 1878 句、clean2 回放：改前改后逐字相同。
- 本机日志：命中数不变，词平均名次 <填实测>。
- 这是 Core 逻辑改动，有单测与回放兜底；壳只改了收件箱一行调用（macOS / 云桥）。

## 注意

- 升级后第一次启动会删掉旧个人英文词表里 ≤ 2 个字母且写法与随包不同的、3–4 个字母又不在随包表的词。
- `user-english.tsv` 多了第三列；旧版本读新文件只取前两列（本 PR 第 2 个提交起就这样读）。
EOF
)"
```

- [ ] **Step 3: 不合并。** 把 PR 链接报给用户，等用户明确说「合 #N」或自己点合并。合了之后：

```bash
git -C /Users/liyuqing/sproot/qingjian worktree remove /Users/liyuqing/sproot/qingjian-english-gate
```

---

## Self-Review

**规格覆盖（d3-learning-gate.md §4 / §6 与任务要求）：**

| 要求 | Task |
|---|---|
| (a) 残段不学（英文候选、回车串、句末英文） | 1 |
| (a) 回车串 < 3 字母不学、< 5 字母且不在随包表不学，原三道过滤保留 | 1 |
| (b) 选择提权不动，只加「两字母英文选两次才排第一」，按随包写法判大小写 | 3 |
| (c) 退格删净撤销英文学习（只退一个字母不撤） | 4 |
| (c) 中文比英文多选 3 次以上英文让到第 5 位（不足 5 个在末尾） | 6 |
| (c) 衰减：次数 < 5 且闲置 > 30 天删，每学一个看 10 条 | 5 |
| (d) 旧 user-english.tsv 加载时清一次，保存后带版本头不再清 | 2 |
| (d) 云同步合并进来的英文词也过闸门 | 2 |
| (a)(d) 与 (b)(c) 分开提交、(a) 最先 | 1、2 在前，3–6 在后 |
| 单测清单 17 条（含 #545 两条：`wolai` 剩 `i`、`sm` 回车；BIP-39 计数照常） | 1（7 条）、2（3 条）、3（2 条）、4（3 条）、6（2 条）；learning 侧 Task 2/4/5 另 7 条 |
| `--replay` 词 920/1057 不变、英文 16/21 → 15/21 写明哪条 | 1 Step 8、7 Step 2（第 1869 行 `ac`） |
| 整句评测不变 | 1、7 |
| docs/user 学习相关页、crate-notes、fork-patch、todo | 每个 Task 的文档步骤 |

**类型一致性**：`EnglishOrigin::{Raw, Selected}`、`english_learnable(&self, &str, EnglishOrigin, bool)`、`raw_english_word(&self, &str)`、`sanitize_user_english_once`、`Engine::merge_remote(&mut self, &str) -> usize`、
`retract_english(&mut self, &str, bool)`、`Learner::{unlearn_english, english_needs_sanitize, english_sanitized}`、`LastCommit::{english, english_recorded}`、
`USER_ENGLISH_VERSION`、`english_legacy` / `english_days` / `english_cursor`、`sweep_english(&mut self, u32) -> usize`、`RetentionPolicy::STATIC_ENGLISH`、
`SHORT_ENGLISH_FIRST_PICKS` / `ENGLISH_DEMOTE_MARGIN` 在定义它们的 Task 与之后使用处名字一致；以上都在临时工作树里编译、测试过。
