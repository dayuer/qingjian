# D1 模糊源拼写当完整音节（修 #319 `tinwosuo`、#306 双拼 `suang`）Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 开了模糊音后，「本身不合法、套一次开着的规则就合法」的拼写（`tin`、`suang`、自然码 `sd`）在切分与双拼解码里当完整音节读，整段纠错只有比模糊读法高出一次编辑代价才接管；模糊音全关时行为逐位不变。

**Architecture:** `FuzzyRules::sources()` 枚举模糊源拼写，`Engine::set_fuzzy` 时建成一张小 `SyllableTrie`（`Engine::fuzzy_sources`）；`parser::segment_with` 在切分 DP 里把这张 trie 的命中也当完整音节（`Syllable::fuzzy = true`），后面的 `FuzzyRules::expand`、查词、整句、上屏消耗都不用改；`correction::unlikely_pinyin` 把模糊音节算「不像话」，让整段纠错照常比较、但原样分已是模糊读法；双拼 `Scheme::decode_with` 用同一张表。两条护栏保证合法拼写的切分与改前逐条一致。

**Tech Stack:** Rust（`qingjian-core`），`qingjian-cli` 的 `--eval-text` / `--replay --clean2 --replay-details` 回归，`tools/release/gate-pass.sh`。

**依据：** 规格 `docs/notes/wetype-impl/d1-fuzzy-as-augmenter.md`（§3.3 改法、§4.2 单测清单）；原型 `docs/notes/wetype-impl/proto/d1-fuzzy-sources.diff` 与 `proto/d1-fuzzy_sources_test.rs`；上游 PR #330 的 `tests/shuangpin_fuzzy.rs`（原样收进本计划 Task 4）。
本计划里的每个补丁都在 `origin/sujian`（= `dd1ce657`）的临时工作树上逐任务打过、`cargo fmt --check` / `cargo clippy --all-targets -D warnings`（根工作区与 `cloud/` 工作区）/ `cargo test -p qingjian-core` 全过，三种模糊配置的评测与回放也跑过（结果见 Task 5）。

---

## 与规格的差异（以代码为准）

1. **`segment_longest_prefix` 不另起 `_with` 版本，直接加参数。** 原型保留无参版本、新增 `segment_longest_prefix_with`；规格 §3.4 要求 `composing.rs` / `prediction/mod.rs` / `marked_rest` 也全部换成带模糊表的版本，换完后无参版本就没有调用方了（clippy 会报 dead_code），所以只留一个 `segment_longest_prefix(text, fuzzy)`。`parser::segment` 仍保留（`phonetic.rs` 的混输头段、外部 crate 在用），新增 `parser::segment_with`。
2. **`FuzzyRules::sources` 放新文件 `fuzzy/sources.rs`。** 原型写在 `rules.rs` 里；按「新代码放新文件、对上游文件只加挂钩」（`cloud/docs/fork-patch.md`）与「大类型的 impl 按职责拆」，`rules.rs` 只把 `apply_once` 拆成 `swaps` + `apply_once` 并标 `pub(super)`。
3. **回放「逐条一致」有一行已知差异。** 规格 §4.3 写「全开 … 完全一致」，实测原型与本计划在 `--fuzzy all` 的回放汇总里都有一行不同：英文那组 `当时纠错生效 2 条，现在仍纠 2 条` → `仍纠 1 条`。原因是日志第 388 行 `serv`（选 service）：改前全开时切成 `se r… v` 不像话，被纠成 `se qv`（无意义的纠正，首选仍是 service）；改后 `rv` 是 l-r 的源拼写（rv→lv），切成 `se rv`，纠正比不过模糊读法、不再纠。`--replay-details` 逐条（名次与前五）**完全相同**，`--eval-text` 完全相同，`in-ing,s-sh` 与全关两种配置回放汇总也完全相同。验收判据据此写成「除这一行外 diff 为空」（Task 5），出现任何别的差异都要停下查。
4. **§4.2「落地时再补」的四组测试全部写成集成测试**，放 `crates/qingjian-core/tests/`（规格写的是 `fuzzy/rules.rs` 单元测试、`parser` 单元测试、`engine/tests/correction.rs`；任务要求测试文件放 `tests/`，且只用公开 API 就能测）。其中「全开时 `hone` / `serv` 英文首选不变」写成「全开与全关时首选相同」的对照（小词库里没有真实英文排序数据，绝对值没意义）。
5. **`prediction/mod.rs` 的 `use super::*;` 不顺手换掉。** contributing 说改到存量 glob 时顺手换，但这是上游文件，换掉要列 17 个名字，徒增与上游的冲突面；本计划只改两处调用，在 fork-patch.md 里写明。

---

## 前置 / 约定（照抄自 PLAN-BRIEF，执行时逐条遵守）

- 执行时开独立工作树：`git -C /Users/liyuqing/sproot/qingjian fetch origin && git -C /Users/liyuqing/sproot/qingjian worktree add ../qingjian-fuzzy -b fuzzy/sources origin/sujian`，所有改动在 `/Users/liyuqing/sproot/qingjian-fuzzy` 里做；主检出不动。做完 PR 合了就删工作树。
- PR 只开到自己的仓库：`gh pr create --repo dayuer/qingjian --base sujian`。**永远不往上游 qingjian-team/qingjian 开 PR、推送任何东西。** `gh` 命令一律带 `--repo dayuer/qingjian`。（规格提到「建议 #330 改成这套机制的用例」——那是上游的事，本计划不碰上游。）
- 提交信息：Conventional Commits，`<类型>(<范围>): <中文说明>`，范围用 crate / 壳名（core、learning、predict、neural、macos、cloud、docs…），正文写为什么。**不要加 Co-Authored-By 或任何署名尾行**（仓库规矩，.githooks/commit-msg 会拦格式）。
- 代码组织：一个类型一个文件；子模块用 `foo/mod.rs` 目录；同前缀兄弟文件收进目录；导入写精确路径不用 glob；单文件 ≤ 800 行，测试超 200 行搬 `tests.rs`；新文件要 `//!` 头；注释中文、只写读代码看不出的约束；不写装饰性分隔注释。详见 docs/contributing.md 与 CLAUDE.md。
- 提交前：`cargo fmt --all`、`cargo clippy --all-targets -- -D warnings`（在 cloud/ 工作区改了就在 cloud/ 下也跑）、相关 crate 的 `cargo test -p <crate>`。本计划不改 `cloud/` 下的代码，但 `cloud/` 依赖 `qingjian-core`，Task 5 照样在 `cloud/` 下 `cargo check --all-targets` 一次。
- 排序 / 整句 / 纠错改动要跑回放与评测：`tools/release/gate-pass.sh` 以及 `--replay` / `--eval-text`；判据见 Task 0 / Task 5。
- 用户能感知的行为变了，同一提交里改 docs/user/ 对应页（本改动不动按键，`keys.md` 不改）；改了 crate 实现要点同步 docs/notes/crate-notes.md。**这两处文档与代码放同一个 PR；为了让每个代码提交都能单独过测试，文档单独一个提交（Task 6），PR squash 合并后落在同一个提交里。**
- 改了上游已有文件要在 cloud/docs/fork-patch.md 加一节记录「分叉补丁」（Task 6）。
- 最后一个 Task 推分支、开 PR，PR 描述写「怎么验证的」。**不合并**（只按用户明确指令合并）。
- 不做（另立项）：整段纠错「换一个字母」不限邻键（#319 的第二层根因，改它会动所有用户的回放）；WeType 有而青简没有的规则（g-k、hui/fei 专用、on→ong 等）。

---

## File Structure

| 文件 | 动作 | 职责 |
|---|---|---|
| `crates/qingjian-core/src/fuzzy/sources.rs` | 新建 | `impl FuzzyRules { pub fn sources() }`：枚举模糊源拼写 |
| `crates/qingjian-core/src/fuzzy/mod.rs` | 改 1 行 | `mod sources;` |
| `crates/qingjian-core/src/fuzzy/rules.rs` | 改 | `apply_once` 拆成 `swaps`（只套规则）+ `apply_once`（过滤合法），都 `pub(super)`；行为不变 |
| `crates/qingjian-core/src/parser/segmentation.rs` | 改 | `Syllable.fuzzy`、`Syllable::fuzzy()`、`Segmentation::fuzzy_count()` |
| `crates/qingjian-core/src/parser/mod.rs` | 改 | `pub use trie::SyllableTrie`；`segment_with`；`segment_chunk` 收模糊 trie；`sort_key` 第二键改「残缺 + 模糊」；合法优先护栏 |
| `crates/qingjian-core/src/engine/mod.rs` | 改 | `Engine::fuzzy_sources` 字段；`segment_longest_prefix(text, fuzzy)`；`marked_rest(rest, fuzzy)` |
| `crates/qingjian-core/src/engine/setup.rs` | 改 | `set_fuzzy` 重建 `fuzzy_sources`；`marked_rest` 传表；`decode` / `takes_semicolon` 用 `decode_with`（Task 4） |
| `crates/qingjian-core/src/engine/query/phonetic.rs`、`correcting.rs`、`commit/mod.rs`、`prediction/mod.rs` | 改调用 | 传 `self.fuzzy_sources.as_ref()` |
| `crates/qingjian-core/src/engine/composing.rs` | 改 | `unit_len_before` / `unit_len_after` 多收 `fuzzy` 参数（按音节删除、移光标） |
| `crates/qingjian-core/src/correction/mod.rs` | 改 | `unlikely_pinyin`：最优切分有模糊音节也算不像话 |
| `crates/qingjian-core/src/shuangpin/scheme.rs` | 改 | `decode_with(keys, fuzzy)` + `fuzzy_syllable`；`decode` = `decode_with(keys, None)` |
| `crates/qingjian-core/tests/fuzzy_sources.rs` | 新建 | 源拼写表、切分（含 `'` 分段、截断、护栏） |
| `crates/qingjian-core/tests/fuzzy_engine.rs` | 新建 | 引擎层：`tinwosuo`、`suang`、`rinshi`、`dakaiho`、英文对照、上屏消耗、按音节移动删除、双拼 |
| `crates/qingjian-core/tests/shuangpin_fuzzy.rs` | 新建 | 上游 PR #330 的三个测试，原样 |
| `docs/user/input/fuzzy-and-shuangpin.md` | 改 | 「不合法的拼写也能走模糊音」 |
| `docs/notes/crate-notes.md` | 改 | 模糊源拼写、两条护栏、纠错比较口径 |
| `cloud/docs/fork-patch.md` | 改 | 新一节「模糊源拼写当完整音节」 |

---

### Task 0: 工作树、评测数据与改前基线

**Files:** 无仓库文件改动（只在 `/tmp/d1-base/`、`/tmp/d1-eval/` 写基线输出；工作树里 `data/` 被 gitignore）

- [ ] **Step 1: 开工作树**

```bash
git -C /Users/liyuqing/sproot/qingjian fetch origin
git -C /Users/liyuqing/sproot/qingjian worktree add ../qingjian-fuzzy -b fuzzy/sources origin/sujian
cd /Users/liyuqing/sproot/qingjian-fuzzy
git log --oneline -1
```

Expected: 打印 `origin/sujian` 的头提交（写计划时是 `dd1ce657`；若已前进，Task 1–4 的 `git apply --check` 会告诉你补丁是否还能打上，打不上就按补丁里的上下文手工改）。

- [ ] **Step 2: 让工作树能跑评测（数据不进 git，从主检出借）**

`data/eval` 与 `data/models` 只读，软链即可；`data/generated` 会被 `gate-pass.sh` 写 `GATE_PASSED`，用 APFS 克隆拷一份，免得写进主检出。

```bash
cd /Users/liyuqing/sproot/qingjian-fuzzy
mkdir -p data
ln -s /Users/liyuqing/sproot/qingjian/data/eval data/eval
ln -s /Users/liyuqing/sproot/qingjian/data/models data/models
cp -cR /Users/liyuqing/sproot/qingjian/data/generated data/generated
git status --short
```

Expected: `git status --short` 无输出（`/data` 在 .gitignore 里）。

- [ ] **Step 3: 编改前的 CLI 并留一份**

```bash
cd /Users/liyuqing/sproot/qingjian-fuzzy
cargo build --release -p qingjian-cli
mkdir -p /tmp/d1-base /tmp/d1-eval
cp target/release/qingjian-cli /tmp/d1-base/qingjian-cli
```

- [ ] **Step 4: 写评测脚本（放仓库外）**

```bash
cat > /tmp/d1-eval/run.sh <<'SH'
#!/usr/bin/env bash
# 用法：run.sh <cli> <标签> [模糊音]；在工作树根目录跑（CLI 按相对路径找 data/generated）
set -uo pipefail
cd /Users/liyuqing/sproot/qingjian-fuzzy
CLI=$1; TAG=$2; FZ=${3:-}; OUT=/tmp/d1-eval
fz=(); [[ -n "$FZ" ]] && fz=(--fuzzy "$FZ")
rm -f "$OUT/details-$TAG.jsonl"
"$CLI" --config tools/eval/offline.toml ${fz[@]+"${fz[@]}"} --eval-text data/eval/sentences.tsv \
  --extra-dict data/generated/dicts/idioms.qj 2>/dev/null > "$OUT/eval-$TAG.txt"
"$CLI" --config tools/eval/offline.toml ${fz[@]+"${fz[@]}"} --replay data/eval/input-log-2026-10-04.jsonl --clean2 \
  --extra-dict data/generated/dicts/idioms.qj --misses 0 --replay-details "$OUT/details-$TAG.jsonl" 2>/dev/null > "$OUT/replay-$TAG.txt"
SH
chmod +x /tmp/d1-eval/run.sh
```

- [ ] **Step 5: 跑改前基线（三种模糊配置）**

```bash
/tmp/d1-eval/run.sh /tmp/d1-base/qingjian-cli base-off
/tmp/d1-eval/run.sh /tmp/d1-base/qingjian-cli base-in-ing_s-sh in-ing,s-sh
/tmp/d1-eval/run.sh /tmp/d1-base/qingjian-cli base-all all
grep -h "^句子" /tmp/d1-eval/eval-base-*.txt
```

Expected（写计划时、本机数据下的数）：全关 `首选 31.5% 前三 36.4% … 字准确率 76.2%`、in-ing,s-sh `30.6% / 35.1% / 75.5%`、全开 `25.8% / 30.6% / 71.6%`。数不同不要紧（数据可能更新过），后面只比改前改后。

- [ ] **Step 6: 跑改前的 gate-pass 并留底**

```bash
cd /Users/liyuqing/sproot/qingjian-fuzzy
tools/release/gate-pass.sh; echo "exit=$?"
for f in /tmp/gate-eval-none.txt /tmp/gate-eval-fusion.txt /tmp/gate-replay-none.txt /tmp/gate-replay-fusion.txt; do cp "$f" /tmp/d1-base/; done
```

Expected: 四个 `/tmp/gate-*.txt` 都在。`exit` 不是 0 也继续（门槛基线随数据走，与本改动无关），把 exit 记下，Task 5 比较的是改前改后输出逐位一致，不是门槛本身。

---

### Task 1: `FuzzyRules::sources` —— 枚举模糊源拼写

**Files:**
- Create: `crates/qingjian-core/src/fuzzy/sources.rs`
- Modify: `crates/qingjian-core/src/fuzzy/mod.rs:8`（加 `mod sources;`）
- Modify: `crates/qingjian-core/src/fuzzy/rules.rs:161-218`（拆 `swaps` / `apply_once`）
- Test: `crates/qingjian-core/tests/fuzzy_sources.rs`

- [ ] **Step 1: 写失败的测试**

新建 `crates/qingjian-core/tests/fuzzy_sources.rs`（Task 2 会把它扩成完整版）：

```rust
//! 只有模糊音才读得通的拼写（`tin` → ting、`suang` → shuang）：源拼写表与切分。

use qingjian_core::FuzzyRules;

fn rules(names: &[&str]) -> FuzzyRules {
    let mut rules = FuzzyRules::default();
    for name in names {
        assert!(rules.enable(name), "{name}");
    }
    rules
}

#[test]
fn sources_are_the_illegal_spellings_one_rule_away() {
    assert_eq!(rules(&["in-ing"]).sources(), ["din", "tin"]);
    assert_eq!(
        rules(&["s-sh"]).sources(),
        ["sei", "shong", "sua", "suai", "suang"]
    );
    assert!(FuzzyRules::default().sources().is_empty());
    assert_eq!(FuzzyRules::ALL.sources().len(), 60);
}

/// 每条规则的受影响面逐条钉住（docs/notes/wetype-impl/d1-fuzzy-as-augmenter.md §3.2），以后改规则悄悄扩大会在这里报。
#[test]
fn sources_match_the_audited_table_rule_by_rule() {
    let table: [(&str, &[&str]); 9] = [
        ("z-zh", &["zua", "zuai", "zuang"]),
        ("c-ch", &["cua", "cuai", "cuang"]),
        ("s-sh", &["sei", "shong", "sua", "suai", "suang"]),
        ("n-l", &["len", "nia", "no"]),
        (
            "f-h",
            &[
                "fai", "fao", "fe", "fong", "fua", "fuai", "fuan", "fuang", "fui", "fun", "fuo",
                "ho",
            ],
        ),
        (
            "l-r",
            &[
                "len", "lua", "lui", "ra", "rai", "rei", "ria", "rian", "riang", "riao", "rie",
                "rin", "ring", "riu", "ro", "rue", "rv", "rve",
            ],
        ),
        (
            "an-ang",
            &[
                "biang", "cuang", "diang", "duang", "juang", "luang", "miang", "nuang", "piang",
                "quang", "ruang", "suang", "tiang", "tuang", "xuang", "yuang", "zuang",
            ],
        ),
        ("en-eng", &["len", "ten"]),
        ("in-ing", &["din", "tin"]),
    ];
    let mut union: Vec<&str> = Vec::new();
    for (name, expected) in table {
        assert_eq!(rules(&[name]).sources(), expected, "{name}");
        union.extend_from_slice(expected);
    }
    union.sort_unstable();
    union.dedup();
    assert_eq!(FuzzyRules::ALL.sources(), union);
}
```

- [ ] **Step 2: 跑测试，确认失败**

Run: `cargo test -p qingjian-core --test fuzzy_sources`
Expected: 编译失败，`no method named `sources` found for struct `FuzzyRules``。

- [ ] **Step 3: 实现**

在工作树根目录打补丁（新建 `fuzzy/sources.rs`，`rules.rs` 只是把过滤挪到 `apply_once`、把「只套规则」拆成 `swaps`，行为不变）：

```bash
cat > /tmp/d1-t1.patch <<'PATCH'
diff --git a/crates/qingjian-core/src/fuzzy/mod.rs b/crates/qingjian-core/src/fuzzy/mod.rs
index 85d71480..0df7c0ed 100644
--- a/crates/qingjian-core/src/fuzzy/mod.rs
+++ b/crates/qingjian-core/src/fuzzy/mod.rs
@@ -6,6 +6,7 @@
 
 mod expanded;
 mod rules;
+mod sources;
 
 pub use expanded::Expanded;
 pub use rules::FuzzyRules;
diff --git a/crates/qingjian-core/src/fuzzy/rules.rs b/crates/qingjian-core/src/fuzzy/rules.rs
index 1cf295f7..d8763c72 100644
--- a/crates/qingjian-core/src/fuzzy/rules.rs
+++ b/crates/qingjian-core/src/fuzzy/rules.rs
@@ -159,7 +159,22 @@ impl FuzzyRules {
     }
 
     /// 对一种写法各套一次规则得到的新写法（可能不合法，这里过滤掉）。
-    fn apply_once(&self, form: &str, complete: bool) -> Vec<String> {
+    pub(super) fn apply_once(&self, form: &str, complete: bool) -> Vec<String> {
+        let mut out = self.swaps(form, complete);
+        out.retain(|f| {
+            !f.is_empty()
+                && f != form
+                && if complete {
+                    parser::is_syllable(f)
+                } else {
+                    parser::is_syllable(f) || parser::is_syllable_prefix(f)
+                }
+        });
+        out
+    }
+
+    /// 各套一次规则，不管结果合不合法。
+    pub(super) fn swaps(&self, form: &str, complete: bool) -> Vec<String> {
         let mut out = Vec::new();
         let mut swap_initial = |a: &str, b: &str| {
             if let Some(rest) = form.strip_prefix(a) {
@@ -206,15 +221,6 @@ impl FuzzyRules {
                 swap_final("ing", "in");
             }
         }
-        out.retain(|f| {
-            !f.is_empty()
-                && f != form
-                && if complete {
-                    parser::is_syllable(f)
-                } else {
-                    parser::is_syllable(f) || parser::is_syllable_prefix(f)
-                }
-        });
         out
     }
 }
diff --git a/crates/qingjian-core/src/fuzzy/sources.rs b/crates/qingjian-core/src/fuzzy/sources.rs
new file mode 100644
index 00000000..b442efd3
--- /dev/null
+++ b/crates/qingjian-core/src/fuzzy/sources.rs
@@ -0,0 +1,29 @@
+//! 模糊源拼写：本身不是合法音节、只有开着的模糊音才读得通的拼写（`tin` → ting、`suang` → shuang）。
+//! 切分与双拼解码把它们也当完整音节，见 docs/notes/wetype-impl/d1-fuzzy-as-augmenter.md。
+
+use super::FuzzyRules;
+use crate::parser;
+
+impl FuzzyRules {
+    /// 开着的规则下的全部模糊源拼写，排好序。规则互为逆变换，所以从合法音节反套一次就能枚举全部；
+    /// 只在改模糊音设置时算一次（全开 60 个）。
+    pub fn sources(&self) -> Vec<String> {
+        let mut out: Vec<String> = Vec::new();
+        if !self.any() {
+            return out;
+        }
+        for syllable in parser::SYLLABLES {
+            for form in self.swaps(syllable, true) {
+                if !parser::is_syllable(&form)
+                    && !out.contains(&form)
+                    && form.len() <= parser::MAX_SYLLABLE_LEN
+                    && !self.apply_once(&form, true).is_empty()
+                {
+                    out.push(form);
+                }
+            }
+        }
+        out.sort();
+        out
+    }
+}
PATCH
git apply --check /tmp/d1-t1.patch && git apply /tmp/d1-t1.patch
```

> 补丁先存到仓库外再打；`--check` 不过（`origin/sujian` 前进过、上下文对不上）就别硬打，按补丁的 `+`/`-` 行手工改同样的位置。

- [ ] **Step 4: 跑测试，确认通过**

Run: `cargo test -p qingjian-core --test fuzzy_sources && cargo test -p qingjian-core --lib fuzzy`
Expected: `fuzzy_sources` 2 passed；`fuzzy` 库测试全过（`apply_once` 行为没变）。

- [ ] **Step 5: 提交**

```bash
cargo fmt --all && cargo clippy -p qingjian-core --all-targets -- -D warnings
git add crates/qingjian-core/src/fuzzy crates/qingjian-core/tests/fuzzy_sources.rs
git commit -m "feat(core): 模糊音算出只靠它才读得通的拼写（FuzzyRules::sources）" -m "tin、suang 这类拼写本身不合法，套一次开着的规则就合法（全开 60 个），后面切分与双拼解码要把它们当完整音节（上游 #319、#306）。
规则互为逆变换，从合法音节反套一次就能枚举全部；放新文件，rules.rs 只把 apply_once 拆出不过滤的 swaps。
测试把每条规则的受影响面逐条钉住，以后改规则悄悄扩大会报。"
```

---

### Task 2: 切分认模糊源拼写（`parser::segment_with`）

**Files:**
- Modify: `crates/qingjian-core/src/parser/segmentation.rs`（`Syllable.fuzzy`、`Syllable::fuzzy`、`Segmentation::fuzzy_count`）
- Modify: `crates/qingjian-core/src/parser/mod.rs`（`pub use trie::SyllableTrie`、`segment_with`、`segment_chunk`、`sort_key`、护栏）
- Test: `crates/qingjian-core/tests/fuzzy_sources.rs`（整文件替换成完整版）

- [ ] **Step 1: 写失败的测试**

把 `crates/qingjian-core/tests/fuzzy_sources.rs` 整个替换为：

```rust
//! 只有模糊音才读得通的拼写（`tin` → ting、`suang` → shuang）：源拼写表与切分。

use qingjian_core::FuzzyRules;
use qingjian_core::parser::{self, SyllableTrie};

fn rules(names: &[&str]) -> FuzzyRules {
    let mut rules = FuzzyRules::default();
    for name in names {
        assert!(rules.enable(name), "{name}");
    }
    rules
}

fn trie(rules: FuzzyRules) -> SyllableTrie {
    let sources = rules.sources();
    let refs: Vec<&str> = sources.iter().map(String::as_str).collect();
    SyllableTrie::build(&refs)
}

fn texts(segmentation: &qingjian_core::Segmentation) -> Vec<(&str, bool, bool)> {
    segmentation
        .syllables
        .iter()
        .map(|s| (s.text.as_str(), s.complete, s.fuzzy))
        .collect()
}

#[test]
fn sources_are_the_illegal_spellings_one_rule_away() {
    assert_eq!(rules(&["in-ing"]).sources(), ["din", "tin"]);
    assert_eq!(
        rules(&["s-sh"]).sources(),
        ["sei", "shong", "sua", "suai", "suang"]
    );
    assert!(FuzzyRules::default().sources().is_empty());
    assert_eq!(FuzzyRules::ALL.sources().len(), 60);
}

/// 每条规则的受影响面逐条钉住（docs/notes/wetype-impl/d1-fuzzy-as-augmenter.md §3.2），以后改规则悄悄扩大会在这里报。
#[test]
fn sources_match_the_audited_table_rule_by_rule() {
    let table: [(&str, &[&str]); 9] = [
        ("z-zh", &["zua", "zuai", "zuang"]),
        ("c-ch", &["cua", "cuai", "cuang"]),
        ("s-sh", &["sei", "shong", "sua", "suai", "suang"]),
        ("n-l", &["len", "nia", "no"]),
        (
            "f-h",
            &[
                "fai", "fao", "fe", "fong", "fua", "fuai", "fuan", "fuang", "fui", "fun", "fuo",
                "ho",
            ],
        ),
        (
            "l-r",
            &[
                "len", "lua", "lui", "ra", "rai", "rei", "ria", "rian", "riang", "riao", "rie",
                "rin", "ring", "riu", "ro", "rue", "rv", "rve",
            ],
        ),
        (
            "an-ang",
            &[
                "biang", "cuang", "diang", "duang", "juang", "luang", "miang", "nuang", "piang",
                "quang", "ruang", "suang", "tiang", "tuang", "xuang", "yuang", "zuang",
            ],
        ),
        ("en-eng", &["len", "ten"]),
        ("in-ing", &["din", "tin"]),
    ];
    let mut union: Vec<&str> = Vec::new();
    for (name, expected) in table {
        assert_eq!(rules(&[name]).sources(), expected, "{name}");
        union.extend_from_slice(expected);
    }
    union.sort_unstable();
    union.dedup();
    assert_eq!(FuzzyRules::ALL.sources(), union);
}

#[test]
fn segmentation_reads_fuzzy_sources_as_complete_but_after_legal_ones() {
    let in_ing = trie(rules(&["in-ing", "s-sh"]));
    let best = &parser::segment_with("tinwosuo", Some(&in_ing)).unwrap()[0];
    assert_eq!(best.to_string(), "tin wo suo");
    assert!(best.syllables[0].fuzzy && !best.syllables[1].fuzzy);
    // 合法切分排第一时与不开模糊音逐条一致
    for input in ["dingan", "dinan", "nihao"] {
        assert_eq!(
            parser::segment_with(input, Some(&in_ing)).unwrap(),
            parser::segment(input).unwrap(),
            "{input}"
        );
    }
    // 末尾没敲完的前缀排在模糊音读法前面：shoufe 是 shou fe…，不是 shou fe→he
    let all = trie(FuzzyRules::ALL);
    assert_eq!(
        parser::segment_with("shoufe", Some(&all)).unwrap()[0].to_string(),
        "shou fe…"
    );
}

#[test]
fn fuzzy_syllables_stay_inside_their_apostrophe_chunk() {
    let in_ing = trie(rules(&["in-ing"]));
    let best = &parser::segment_with("tin'wo", Some(&in_ing)).unwrap()[0];
    assert_eq!(texts(best), [("tin", true, true), ("wo", true, false)]);
    // `'` 把 tin 拆开了：ti 与 nwo 各自切，不会拼回 tin
    assert_eq!(
        parser::segment_with("ti'nwo", Some(&in_ing)).unwrap(),
        parser::segment("ti'nwo").unwrap()
    );
}

#[test]
fn legal_segmentation_survives_truncation_when_the_best_is_fuzzy() {
    let trie = trie(rules(&["in-ing", "s-sh"]));
    let all = parser::segment_with("tinwosuo", Some(&trie)).unwrap();
    assert!(all.len() <= 8);
    assert!(
        all.iter().any(|s| texts(s)
            == [
                ("ti", true, false),
                ("n", false, false),
                ("wo", true, false),
                ("suo", true, false)
            ]),
        "{all:?}"
    );
}
```

- [ ] **Step 2: 跑测试，确认失败**

Run: `cargo test -p qingjian-core --test fuzzy_sources`
Expected: 编译失败，`parser::SyllableTrie` 私有 / `segment_with` 不存在 / `Syllable` 没有 `fuzzy` 字段。

- [ ] **Step 3: 实现**

要点（补丁里都有，读一遍再打）：
- `segment_chunk` 在合法音节长度之外，把 `fuzzy.matches(rest)` 的长度在**没有同长合法完整音节**时以 `(len, complete=true, only_fuzzy=true)` 入 DP，生成 `Syllable::fuzzy(text)`。
- 护栏 1（排序）：`sort_key` 第二键是「残缺数 + 模糊数」、第三键是模糊数。否则全开时 `shoufe` 的 `fe`（fe→he）会以完整音节压过末尾前缀 `fe…`，收费 掉到第 2。
- 护栏 2（合法优先）：带模糊表切完后，最优切分不含模糊音节就整段改用 `segment_with(input, None)` 的结果。否则 `dingan`（in-ing）会多出 `din gan` 一路，词候选第 2、3 位从 定安镇 / 定 变成 定岗 / 顶岗。代价：`dinan` 想要 定案 的人得不到它（WeType 能），换来合法拼写的候选与不开模糊音逐条一致。

```bash
cat > /tmp/d1-t2.patch <<'PATCH'
diff --git a/crates/qingjian-core/src/parser/mod.rs b/crates/qingjian-core/src/parser/mod.rs
index 463a5e01..8233ccc1 100644
--- a/crates/qingjian-core/src/parser/mod.rs
+++ b/crates/qingjian-core/src/parser/mod.rs
@@ -14,6 +14,7 @@ use std::cmp::Reverse;
 pub use error::ParseError;
 pub use segmentation::{Segmentation, Syllable};
 pub use syllable::{INITIALS, MAX_SYLLABLE_LEN, SYLLABLES};
+pub use trie::SyllableTrie;
 
 use syllable::initial_lengths;
 use trie::SYLLABLE_TRIE;
@@ -52,6 +53,15 @@ const MAX_SEGMENTATIONS: usize = 8;
 
 /// 切分。返回按「音节少、不完整音节少、前面的音节长」排序的切分，最多 [`MAX_SEGMENTATIONS`] 种。
 pub fn segment(input: &str) -> Result<Vec<Segmentation>, ParseError> {
+    segment_with(input, None)
+}
+
+/// 同 [`segment`]，`fuzzy` 是只有模糊音才读得通的拼写（[`crate::FuzzyRules::sources`] 建的 trie）：
+/// 它们也当完整音节切，排在同音节数、同残缺数的合法切分后面。
+pub fn segment_with(
+    input: &str,
+    fuzzy: Option<&SyllableTrie>,
+) -> Result<Vec<Segmentation>, ParseError> {
     let input = input.trim();
     if input.is_empty() {
         return Err(ParseError::Empty);
@@ -75,7 +85,7 @@ pub fn segment(input: &str) -> Result<Vec<Segmentation>, ParseError> {
     }];
     for (index, chunk) in chunks.iter().enumerate() {
         let is_last = index + 1 == chunks.len();
-        let options = segment_chunk(chunk, is_last);
+        let options = segment_chunk(chunk, is_last, fuzzy);
         if options.is_empty() {
             return Err(ParseError::NoSegmentation);
         }
@@ -90,6 +100,10 @@ pub fn segment(input: &str) -> Result<Vec<Segmentation>, ParseError> {
         results = next;
     }
     prune(&mut results);
+    // 最优切分用不着模糊音时，别的切分也不用：合法拼写的候选与不开模糊音时逐条一致（`dingan` 不冒出 `din gan` 的 定岗）
+    if fuzzy.is_some() && results.first().is_some_and(|best| best.fuzzy_count() == 0) {
+        return segment_with(input, None);
+    }
     Ok(results)
 }
 
@@ -100,11 +114,13 @@ fn prune(segmentations: &mut Vec<Segmentation>) {
     segmentations.truncate(MAX_SEGMENTATIONS);
 }
 
-/// 音节少的优先；同音节数时不完整音节少的优先；再同则前面的音节越长越优先（贪心的结果排最前）。
-fn sort_key(segmentation: &Segmentation) -> (usize, usize, Vec<Reverse<usize>>) {
+/// 音节少的优先；同音节数时「不完整 + 只靠模糊音读通」的音节少的优先（`shoufe` 的 `fe…` 前缀排在模糊音的 `fe`→he 前面），
+/// 再同时只靠模糊音的少的优先；再同则前面的音节越长越优先（贪心的结果排最前）。
+fn sort_key(segmentation: &Segmentation) -> (usize, usize, usize, Vec<Reverse<usize>>) {
     (
         segmentation.syllables.len(),
-        segmentation.incomplete_count(),
+        segmentation.incomplete_count() + segmentation.fuzzy_count(),
+        segmentation.fuzzy_count(),
         segmentation
             .syllables
             .iter()
@@ -115,7 +131,11 @@ fn sort_key(segmentation: &Segmentation) -> (usize, usize, Vec<Reverse<usize>>)
 
 /// 切分不含 `'` 的一段：按位置做动态规划，每个位置只保留最优的几种前缀切分。
 /// `allow_partial` 为真时末尾允许留一个残缺音节。
-fn segment_chunk(chunk: &str, allow_partial: bool) -> Vec<Segmentation> {
+fn segment_chunk(
+    chunk: &str,
+    allow_partial: bool,
+    fuzzy: Option<&SyllableTrie>,
+) -> Vec<Segmentation> {
     let n = chunk.len();
     let mut best: Vec<Vec<Segmentation>> = vec![Vec::new(); n + 1];
     best[0].push(Segmentation {
@@ -128,11 +148,19 @@ fn segment_chunk(chunk: &str, allow_partial: bool) -> Vec<Segmentation> {
         prune(&mut best[start]);
         let rest = &chunk[start..];
         let matches = SYLLABLE_TRIE.matches(rest);
-        // (长度, 是否完整音节)
-        let mut tokens: Vec<(usize, bool)> = matches.lengths().map(|len| (len, true)).collect();
+        // (长度, 是否完整音节, 是否只靠模糊音)
+        let mut tokens: Vec<(usize, bool, bool)> =
+            matches.lengths().map(|len| (len, true, false)).collect();
+        if let Some(fuzzy) = fuzzy {
+            for len in fuzzy.matches(rest).lengths() {
+                if !tokens.iter().any(|t| t.0 == len && t.1) {
+                    tokens.push((len, true, true));
+                }
+            }
+        }
         for len in initial_lengths(rest) {
-            if !tokens.contains(&(len, true)) {
-                tokens.push((len, false));
+            if !tokens.iter().any(|t| t.0 == len && t.1) {
+                tokens.push((len, false, false));
             }
         }
         // 整个剩余部分作为未打完的音节（`zho` → zhong / zhou …）。它本身是完整音节或纯声母时已经在上面了。
@@ -140,17 +168,19 @@ fn segment_chunk(chunk: &str, allow_partial: bool) -> Vec<Segmentation> {
         if allow_partial
             && matches.whole_is_prefix
             && !rest_is_complete
-            && !tokens.contains(&(rest.len(), false))
+            && !tokens.iter().any(|t| t.0 == rest.len() && !t.1)
         {
-            tokens.push((rest.len(), false));
+            tokens.push((rest.len(), false, false));
         }
-        for (len, complete) in tokens {
+        for (len, complete, only_fuzzy) in tokens {
             let text = &rest[..len];
             let extended: Vec<Segmentation> = best[start]
                 .iter()
                 .map(|base| {
                     let mut syllables = base.syllables.clone();
-                    syllables.push(if complete {
+                    syllables.push(if only_fuzzy {
+                        Syllable::fuzzy(text)
+                    } else if complete {
                         Syllable::complete(text)
                     } else {
                         Syllable::partial(text)
diff --git a/crates/qingjian-core/src/parser/segmentation.rs b/crates/qingjian-core/src/parser/segmentation.rs
index 8e154420..13a91f44 100644
--- a/crates/qingjian-core/src/parser/segmentation.rs
+++ b/crates/qingjian-core/src/parser/segmentation.rs
@@ -10,6 +10,9 @@ pub struct Syllable {
 
     /// 是完整音节；否则是声母（简拼 `k`）或未打完的前缀（`zho`）。
     pub complete: bool,
+
+    /// 敲的拼写本身不是合法音节，只有开着的模糊音能把它读成合法音节（`tin` → ting、`suang` → shuang）。
+    pub fuzzy: bool,
 }
 
 impl Syllable {
@@ -17,6 +20,16 @@ impl Syllable {
         Self {
             text: text.to_owned(),
             complete: true,
+            fuzzy: false,
+        }
+    }
+
+    /// 只有模糊音才读得通的完整音节。
+    pub fn fuzzy(text: &str) -> Self {
+        Self {
+            text: text.to_owned(),
+            complete: true,
+            fuzzy: true,
         }
     }
 
@@ -24,6 +37,7 @@ impl Syllable {
         Self {
             text: text.to_owned(),
             complete: false,
+            fuzzy: false,
         }
     }
 
@@ -56,6 +70,11 @@ impl Segmentation {
         self.syllables.iter().filter(|s| !s.complete).count()
     }
 
+    /// 只有模糊音才读得通的音节个数。
+    pub fn fuzzy_count(&self) -> usize {
+        self.syllables.iter().filter(|s| s.fuzzy).count()
+    }
+
     pub fn last_is_partial(&self) -> bool {
         self.syllables.last().is_some_and(|s| !s.complete)
     }
PATCH
git apply --check /tmp/d1-t2.patch && git apply /tmp/d1-t2.patch
```

- [ ] **Step 4: 跑测试，确认通过**

Run: `cargo test -p qingjian-core --test fuzzy_sources && cargo test -p qingjian-core --lib parser`
Expected: `fuzzy_sources` 5 passed；`parser` 库测试 15 passed（`segment` = `segment_with(_, None)`，旧行为不变）。

- [ ] **Step 5: 提交**

```bash
cargo fmt --all && cargo clippy -p qingjian-core --all-targets -- -D warnings
git add crates/qingjian-core/src/parser crates/qingjian-core/tests/fuzzy_sources.rs
git commit -m "feat(core): 切分可以把模糊源拼写当完整音节（parser::segment_with）" -m "青简的模糊音是先切合法音节再替换，tin 切成 ti n…，模糊音根本没机会作用；WeType 在音节 trie 上做按键级变换，源拼写合不合法无所谓。
这里多给切分 DP 一张模糊源拼写的小 trie，命中的长度也当完整音节（Syllable::fuzzy），查词照旧走 FuzzyRules::expand。
两条护栏：排序第二键改成残缺 + 模糊（shoufe 仍是 shou fe…）；最优切分用不着模糊音时整段退回不带模糊表的切分（dingan 的候选与不开模糊音逐条一致）。"
```

---

### Task 3: 引擎接上模糊源拼写，整段纠错要比模糊读法好一次编辑代价才接管

**Files:**
- Modify: `crates/qingjian-core/src/engine/mod.rs`（字段、`segment_longest_prefix`、`marked_rest`）
- Modify: `crates/qingjian-core/src/engine/setup.rs`（`set_fuzzy` 重建表、`marked_rest` 传表）
- Modify: `crates/qingjian-core/src/engine/query/phonetic.rs:35`、`engine/correcting.rs:44`、`engine/commit/mod.rs:452`、`engine/prediction/mod.rs:102,339`（传表）
- Modify: `crates/qingjian-core/src/engine/composing.rs`（`unit_len_before` / `unit_len_after` 多收 `fuzzy`）
- Modify: `crates/qingjian-core/src/correction/mod.rs:45-54`（`unlikely_pinyin`）
- Test: `crates/qingjian-core/tests/fuzzy_engine.rs`

- [ ] **Step 1: 写失败的测试**

新建 `crates/qingjian-core/tests/fuzzy_engine.rs`（双拼那条在 Task 4 加）：

```rust
//! 模糊源拼写在引擎里：查询、整段纠错让位、上屏消耗、按音节移动删除（上游 #319）。

use qingjian_core::{Engine, FuzzyRules};
use qingjian_dictionary::{Dictionary, WordList};

const DICT: &str = "听\tting\t9000\n我\two\t9000\n说\tshuo\t9000\n请\tqing\t8000\n亲\tqin\t8000\n\
替\tti\t5000\n你\tni\t9000\n所\tsuo\t500\n双\tshuang\t9000\n人\tren\t8000\n双人\tshuang ren\t12000\n\
定\tding\t9000\n案\tan\t5000\n定案\tding an\t8000\n岗\tgang\t3000\n定岗\tding gang\t2000\n";

fn rules(names: &[&str]) -> FuzzyRules {
    let mut rules = FuzzyRules::default();
    for name in names {
        assert!(rules.enable(name), "{name}");
    }
    rules
}

fn engine(dict: &str) -> Engine {
    Engine::new(Dictionary::parse(dict).unwrap())
}

fn first(engine: &mut Engine, keys: &str) -> String {
    engine.set_input(keys);
    engine.query().unwrap().candidates.items[0].text.clone()
}

fn offers(engine: &mut Engine, keys: &str, text: &str) -> bool {
    engine.set_input(keys);
    engine
        .query()
        .is_ok_and(|q| q.candidates.items.iter().any(|c| c.text == text))
}

#[test]
fn tinwosuo_is_read_by_fuzzy_rules_not_corrected() {
    let mut engine = engine(DICT);
    engine.set_fuzzy(rules(&["in-ing", "s-sh"]));
    assert_eq!(first(&mut engine, "tinwosuo"), "听我说");
    assert!(engine.query().unwrap().correction.is_none());
    // 关掉模糊音：行为同改前（不出 听我说）
    engine.set_fuzzy(FuzzyRules::default());
    assert_ne!(first(&mut engine, "tinwosuo"), "听我说");
    // 合法拼写不受影响
    engine.set_fuzzy(rules(&["in-ing"]));
    assert_eq!(first(&mut engine, "dingan"), "定案");
}

#[test]
fn suang_needs_s_sh() {
    let mut engine = engine(DICT);
    engine.set_fuzzy(rules(&["s-sh"]));
    assert_eq!(first(&mut engine, "suang"), "双");
    engine.set_fuzzy(rules(&["n-l"]));
    assert!(!offers(&mut engine, "suang", "双"));
}

#[test]
fn committing_a_fuzzy_word_consumes_its_typed_keys() {
    let mut engine = engine(DICT);
    engine.set_fuzzy(rules(&["in-ing", "s-sh"]));
    engine.set_input("tinwosuo");
    let query = engine.query().unwrap();
    let ting = query
        .candidates
        .items
        .iter()
        .find(|c| c.text == "听")
        .unwrap()
        .clone();
    assert_eq!(engine.commit(&ting), "听");
    assert_eq!(engine.composition().text(), "wosuo");
}

#[test]
fn syllable_moves_and_deletes_treat_a_fuzzy_spelling_as_one_syllable() {
    let mut engine = engine(DICT);
    engine.set_fuzzy(rules(&["in-ing", "s-sh"]));
    engine.set_input("tinwosuo");
    engine.move_cursor_home();
    assert!(engine.move_cursor_syllable_right());
    assert_eq!(engine.composition().cursor(), 3, "tin 是一个音节，不是 ti");
    engine.move_cursor_end();
    for rest in ["tinwo", "tin", ""] {
        assert!(engine.delete_syllable_backward());
        assert_eq!(engine.composition().text(), rest);
    }
}

#[test]
fn a_better_correction_still_wins_over_a_fuzzy_reading() {
    // 全开时 ho 是 f-h 的源拼写（ho → fo），但 dakaihou 少敲一个 u 的纠正比 打开佛 好得多
    let mut engine = engine(
        "打开\tda kai\t9000\n打\tda\t5000\n开\tkai\t5000\n后\thou\t9000\n打开后\tda kai hou\t3000\n佛\tfo\t100\n",
    );
    engine.set_fuzzy(FuzzyRules::ALL);
    assert_eq!(first(&mut engine, "dakaiho"), "打开后");
}

#[test]
fn a_fuzzy_reading_beats_a_one_edit_correction() {
    // l-r：rin 只靠模糊音读成 lin；ln 2 的模糊代价小于一次编辑，临时 胜过纠成 renshi 的 认识
    let mut engine = engine("临时\tlin shi\t5000\n认识\tren shi\t9000\n");
    engine.set_fuzzy(rules(&["l-r"]));
    assert_eq!(first(&mut engine, "rinshi"), "临时");
    assert!(engine.query().unwrap().correction.is_none());
}

#[test]
fn english_words_keep_their_place_with_every_rule_on() {
    let words = "hone\nservice\nserve\n";
    let dict = "和\the\t9000\n呢\tne\t9000\n色\tse\t5000\n";
    let mut plain =
        Engine::new(Dictionary::parse(dict).unwrap()).with_english(WordList::parse(words).unwrap());
    let mut fuzzy =
        Engine::new(Dictionary::parse(dict).unwrap()).with_english(WordList::parse(words).unwrap());
    fuzzy.set_fuzzy(FuzzyRules::ALL);
    for keys in ["hone", "serv"] {
        assert_eq!(first(&mut fuzzy, keys), first(&mut plain, keys), "{keys}");
    }
}
```

- [ ] **Step 2: 跑测试，确认失败**

Run: `cargo test -p qingjian-core --test fuzzy_engine`
Expected: 编译通过（只用公开 API），**4 个失败**：`tinwosuo_is_read_by_fuzzy_rules_not_corrected`（首选 请我说 类的纠正）、`suang_needs_s_sh`、`a_fuzzy_reading_beats_a_one_edit_correction`（首选 认识）、`syllable_moves_and_deletes_treat_a_fuzzy_spelling_as_one_syllable`（光标停在 2）；
`committing_a_fuzzy_word_consumes_its_typed_keys`、`a_better_correction_still_wins_over_a_fuzzy_reading`、`english_words_keep_their_place_with_every_rule_on` 改前就过——它们是护栏，改后必须仍过。

- [ ] **Step 3: 实现**

要点：
- `Engine` 加 `fuzzy_sources: Option<parser::SyllableTrie>`，`set_fuzzy` 在规则变化时用 `rules.sources()` 重建（全关为 `None`，所以全关时所有路径都等价于改前）。
- `segment_longest_prefix(text, fuzzy)` 一个函数，所有调用方传 `self.fuzzy_sources.as_ref()`：查询、整段纠错、上屏后整句对照、联想、`marked_rest`、按音节删除 / 移光标（`composing.rs` 的两个自由函数多收一个参数）。
- `correction::unlikely_pinyin`：最优切分里有模糊音节也算不像话 → 整段纠错照样生成候选，但原样分取的是最优切分（就是模糊读法，整句分已含每个模糊音节 ln 2 的代价），纠正仍须 `score − 5.0 > raw` 才接管。
  `tinwosuo`：原样 听我说 约 −16.7，最好的纠正 请我说 −19.5，不纠。保留纠错入口是因为若模糊音节直接算「像话」，全开时 `dakaiho`（`ho→fo`）就不再纠成 打开后。

```bash
cat > /tmp/d1-t3.patch <<'PATCH'
diff --git a/crates/qingjian-core/src/correction/mod.rs b/crates/qingjian-core/src/correction/mod.rs
index 5591a721..a4943bbe 100644
--- a/crates/qingjian-core/src/correction/mod.rs
+++ b/crates/qingjian-core/src/correction/mod.rs
@@ -42,14 +42,17 @@ pub fn eligible(input: &str) -> bool {
 
 /// 切出来的拼音「不像话」：有切不动的尾巴，或非末尾的音节里有简拼 / 残缺。
 /// 英文补全与拼写纠错都用这个判断。
+/// 有只靠模糊音读通的音节（`tin` → ting）也算：纠错照样试，但要比模糊音读法的整句分高出一次编辑的代价才纠。
 pub fn unlikely_pinyin(best: Option<&Segmentation>, tail: &str) -> bool {
     !tail.is_empty()
         || best.is_some_and(|best| {
             let count = best.syllables.len();
-            best.syllables
-                .iter()
-                .take(count.saturating_sub(1))
-                .any(|s| !s.complete)
+            best.syllables.iter().any(|s| s.fuzzy)
+                || best
+                    .syllables
+                    .iter()
+                    .take(count.saturating_sub(1))
+                    .any(|s| !s.complete)
         })
 }
 
diff --git a/crates/qingjian-core/src/engine/commit/mod.rs b/crates/qingjian-core/src/engine/commit/mod.rs
index 00630237..6d7dce49 100644
--- a/crates/qingjian-core/src/engine/commit/mod.rs
+++ b/crates/qingjian-core/src/engine/commit/mod.rs
@@ -449,7 +449,8 @@ impl Engine {
             }
             (None, Some(c)) => self.convert_sentence(&c.segmentation.patterns(), false)?,
             (None, None) => {
-                let (segmentations, _) = segment_longest_prefix(scope).ok()?;
+                let (segmentations, _) =
+                    segment_longest_prefix(scope, self.fuzzy_sources.as_ref()).ok()?;
                 self.convert_sentence(&segmentations.first()?.patterns(), true)?
             }
         };
diff --git a/crates/qingjian-core/src/engine/composing.rs b/crates/qingjian-core/src/engine/composing.rs
index b35707f7..19fe8f62 100644
--- a/crates/qingjian-core/src/engine/composing.rs
+++ b/crates/qingjian-core/src/engine/composing.rs
@@ -9,6 +9,7 @@ use super::{
 };
 use crate::composition::Composition;
 use crate::correction;
+use crate::parser::SyllableTrie;
 use crate::shortcut;
 use std::time::Instant;
 
@@ -238,7 +239,12 @@ impl Engine {
         let before = &self.composition.text()[..cursor];
         let plain =
             self.raw_mode() || self.expression_mode() || self.question_mode() || self.zhuyin;
-        let len = unit_len_before(before, self.shuangpin.is_some(), plain);
+        let len = unit_len_before(
+            before,
+            self.shuangpin.is_some(),
+            plain,
+            self.fuzzy_sources.as_ref(),
+        );
         self.composition.delete_before_cursor(len)
     }
 
@@ -248,7 +254,12 @@ impl Engine {
         let before = &self.composition.text()[..cursor];
         let plain =
             self.raw_mode() || self.expression_mode() || self.question_mode() || self.zhuyin;
-        let len = unit_len_before(before, self.shuangpin.is_some(), plain);
+        let len = unit_len_before(
+            before,
+            self.shuangpin.is_some(),
+            plain,
+            self.fuzzy_sources.as_ref(),
+        );
         len > 0 && (0..len).all(|_| self.composition.move_left())
     }
 
@@ -258,7 +269,12 @@ impl Engine {
         let after = &self.composition.text()[cursor..];
         let plain =
             self.raw_mode() || self.expression_mode() || self.question_mode() || self.zhuyin;
-        let len = unit_len_after(after, self.shuangpin.is_some(), plain);
+        let len = unit_len_after(
+            after,
+            self.shuangpin.is_some(),
+            plain,
+            self.fuzzy_sources.as_ref(),
+        );
         len > 0 && (0..len).all(|_| self.composition.move_right())
     }
 
@@ -398,7 +414,12 @@ impl Engine {
 }
 
 /// 光标后的第一个「单位」占几个字节：先跳过紧跟的 `'`，再算一个音节；规则同 [`unit_len_before`]。
-fn unit_len_after(after: &str, shuangpin: bool, plain: bool) -> usize {
+fn unit_len_after(
+    after: &str,
+    shuangpin: bool,
+    plain: bool,
+    fuzzy: Option<&SyllableTrie>,
+) -> usize {
     let trimmed = after.trim_start_matches('\'');
     let separators = after.len() - trimmed.len();
     let Some(first) = trimmed.chars().next() else {
@@ -423,7 +444,7 @@ fn unit_len_after(after: &str, shuangpin: bool, plain: bool) -> usize {
             .count();
         return separators + run.min(2);
     }
-    let syllable = match segment_longest_prefix(trimmed) {
+    let syllable = match segment_longest_prefix(trimmed, fuzzy) {
         Ok((segmentations, _)) => segmentations
             .first()
             .and_then(|s| s.syllables.first())
@@ -434,7 +455,12 @@ fn unit_len_after(after: &str, shuangpin: bool, plain: bool) -> usize {
 }
 
 /// 光标前的最后一个「单位」占几个字节：拼音里是一个音节（连同它后面的 `'`），见 [`Engine::delete_syllable_backward`]。
-fn unit_len_before(before: &str, shuangpin: bool, plain: bool) -> usize {
+fn unit_len_before(
+    before: &str,
+    shuangpin: bool,
+    plain: bool,
+    fuzzy: Option<&SyllableTrie>,
+) -> usize {
     let trimmed = before.trim_end_matches('\'');
     let separators = before.len() - trimmed.len();
     let Some(last) = trimmed.chars().last() else {
@@ -463,7 +489,7 @@ fn unit_len_before(before: &str, shuangpin: bool, plain: bool) -> usize {
             .count();
         return separators + if run % 2 == 1 { 1 } else { 2 };
     }
-    let syllable = match segment_longest_prefix(trimmed) {
+    let syllable = match segment_longest_prefix(trimmed, fuzzy) {
         Ok((_, tail)) if !tail.is_empty() => tail.len(),
         Ok((segmentations, _)) => segmentations
             .first()
diff --git a/crates/qingjian-core/src/engine/correcting.rs b/crates/qingjian-core/src/engine/correcting.rs
index ebbd1ca6..05d6c907 100644
--- a/crates/qingjian-core/src/engine/correcting.rs
+++ b/crates/qingjian-core/src/engine/correcting.rs
@@ -41,7 +41,8 @@ impl Engine {
         {
             return None;
         }
-        let (segmentations, tail) = segment_longest_prefix(scope).ok()?;
+        let (segmentations, tail) =
+            segment_longest_prefix(scope, self.fuzzy_sources.as_ref()).ok()?;
         // 不像话的拼音试全部一处编辑；末尾单字母的只试相邻换位（`mingtain` → `mingtian`），其余合法拼音不碰
         let candidates = if correction::unlikely_pinyin(segmentations.first(), tail) {
             correction::candidates(scope)
diff --git a/crates/qingjian-core/src/engine/mod.rs b/crates/qingjian-core/src/engine/mod.rs
index 93312c05..731f9ea7 100644
--- a/crates/qingjian-core/src/engine/mod.rs
+++ b/crates/qingjian-core/src/engine/mod.rs
@@ -287,6 +287,9 @@ pub struct Engine {
     /// 模糊音开关，缺省全关。
     fuzzy: FuzzyRules,
 
+    /// 只有开着的模糊音才读得通的拼写（`tin`、`suang`），切分时也当完整音节；模糊音全关时为 `None`。
+    fuzzy_sources: Option<parser::SyllableTrie>,
+
     /// 双拼方案，`None` 为全拼。开着时缓冲区里是双拼键，查词前先解成全拼（见 [`crate::shuangpin`]）。
     shuangpin: Option<Scheme>,
 
@@ -517,6 +520,7 @@ impl Engine {
             last_question_guess: String::new(),
             chain: CommitChain::default(),
             fuzzy: FuzzyRules::default(),
+            fuzzy_sources: None,
             shuangpin: None,
             shuangpin_raw_preedit: false,
             zhuyin: false,
@@ -593,11 +597,11 @@ fn take_first_chars(text: &str, count: usize) -> String {
 }
 
 /// 光标后剩余拼音的显示形式：能切就按音节用 `'` 连上，切不动就原样。
-fn marked_rest(rest: &str) -> String {
+fn marked_rest(rest: &str, fuzzy: Option<&parser::SyllableTrie>) -> String {
     if rest.is_empty() {
         return String::new();
     }
-    match segment_longest_prefix(rest) {
+    match segment_longest_prefix(rest, fuzzy) {
         Ok((segmentations, tail)) => query::join_marked(&segmentations, tail),
         Err(_) => rest.to_owned(),
     }
@@ -605,13 +609,17 @@ fn marked_rest(rest: &str) -> String {
 
 /// 整段切不动时退而求其次：找能切分的最长前缀，剩余字母作为尾部返回。
 /// `kaifv` → (`kai f…` 的切分, `v`)。连第一个字母都切不动才报错。
-fn segment_longest_prefix(text: &str) -> Result<(Vec<Segmentation>, &str), ParseError> {
-    match parser::segment(text) {
+/// `fuzzy` 是只有开着的模糊音才读得通的拼写（[`Engine`] 的 `fuzzy_sources`），切分时也当完整音节（见 [`parser::segment_with`]）。
+fn segment_longest_prefix<'a>(
+    text: &'a str,
+    fuzzy: Option<&parser::SyllableTrie>,
+) -> Result<(Vec<Segmentation>, &'a str), ParseError> {
+    match parser::segment_with(text, fuzzy) {
         Ok(segmentations) => Ok((segmentations, "")),
         Err(ParseError::NoSegmentation) => (1..text.len())
             .rev()
             .find_map(|end| {
-                parser::segment(&text[..end])
+                parser::segment_with(&text[..end], fuzzy)
                     .ok()
                     .map(|s| (s, &text[end..]))
             })
diff --git a/crates/qingjian-core/src/engine/prediction/mod.rs b/crates/qingjian-core/src/engine/prediction/mod.rs
index 61ebc02d..ed4de118 100644
--- a/crates/qingjian-core/src/engine/prediction/mod.rs
+++ b/crates/qingjian-core/src/engine/prediction/mod.rs
@@ -99,18 +99,19 @@ impl Engine {
         if letters < MIN_PREDICTION_LETTERS {
             return None;
         }
-        let (pinyin, syllables, guess, abbreviated) = match segment_longest_prefix(pinyin_source) {
-            Ok((segmentations, tail)) => {
-                let best = segmentations.first();
-                (
-                    query::join_marked(&segmentations, tail),
-                    best.map_or(0, |s| s.syllables.len()),
-                    self.local_guess(&segmentations),
-                    best.is_some_and(mostly_abbreviated),
-                )
-            }
-            Err(_) => (pinyin_source.to_owned(), 0, String::new(), false),
-        };
+        let (pinyin, syllables, guess, abbreviated) =
+            match segment_longest_prefix(pinyin_source, self.fuzzy_sources.as_ref()) {
+                Ok((segmentations, tail)) => {
+                    let best = segmentations.first();
+                    (
+                        query::join_marked(&segmentations, tail),
+                        best.map_or(0, |s| s.syllables.len()),
+                        self.local_guess(&segmentations),
+                        best.is_some_and(mostly_abbreviated),
+                    )
+                }
+                Err(_) => (pinyin_source.to_owned(), 0, String::new(), false),
+            };
         if question {
             self.last_question_guess = guess.clone();
         }
@@ -336,7 +337,7 @@ impl Engine {
         let typed = decoded
             .as_ref()
             .map_or(self.composition.scope(), |d| d.pinyin());
-        if let Ok((segmentations, "")) = segment_longest_prefix(typed)
+        if let Ok((segmentations, "")) = segment_longest_prefix(typed, self.fuzzy_sources.as_ref())
             && let Some(best) = segmentations.first()
             && best.syllables.len() == chars.len()
             && best
diff --git a/crates/qingjian-core/src/engine/query/phonetic.rs b/crates/qingjian-core/src/engine/query/phonetic.rs
index 81648628..62cf26fe 100644
--- a/crates/qingjian-core/src/engine/query/phonetic.rs
+++ b/crates/qingjian-core/src/engine/query/phonetic.rs
@@ -32,7 +32,7 @@ impl Engine {
             (None, Some(tail)) if head_wins => {
                 parser::segment(&keys[..tail.head_len]).map(|s| (s, ""))
             }
-            _ => segment_longest_prefix(keys),
+            _ => segment_longest_prefix(keys, self.fuzzy_sources.as_ref()),
         };
         // 连第一个字母都切不动（`impor`）：拼音这边没戏，但英文词 / 补全、快捷候选还可以有
         let (segmentations, tail) = match parsed {
diff --git a/crates/qingjian-core/src/engine/setup.rs b/crates/qingjian-core/src/engine/setup.rs
index 1d8f274f..7a7fb2e7 100644
--- a/crates/qingjian-core/src/engine/setup.rs
+++ b/crates/qingjian-core/src/engine/setup.rs
@@ -217,7 +217,7 @@ impl Engine {
         }
         match self.decode(rest) {
             Some(decoded) => decoded.marked(),
-            None => marked_rest(rest),
+            None => marked_rest(rest, self.fuzzy_sources.as_ref()),
         }
     }
 
@@ -235,6 +235,11 @@ impl Engine {
     pub fn set_fuzzy(&mut self, rules: FuzzyRules) {
         if self.fuzzy != rules {
             self.forget_span_cache();
+            let sources = rules.sources();
+            self.fuzzy_sources = (!sources.is_empty()).then(|| {
+                let refs: Vec<&str> = sources.iter().map(String::as_str).collect();
+                crate::parser::SyllableTrie::build(&refs)
+            });
         }
         self.fuzzy = rules;
     }
PATCH
git apply --check /tmp/d1-t3.patch && git apply /tmp/d1-t3.patch
```

- [ ] **Step 4: 跑测试，确认通过**

Run: `cargo test -p qingjian-core`
Expected: 库测试 363 passed（数字随上游变，关键是 0 failed）；`fuzzy_engine` 7 passed；`fuzzy_sources` 5 passed；其余集成测试全过。

- [ ] **Step 5: 提交**

```bash
cargo fmt --all && cargo clippy -p qingjian-core --all-targets -- -D warnings
git add crates/qingjian-core/src/engine crates/qingjian-core/src/correction crates/qingjian-core/tests/fuzzy_engine.rs
git commit -m "fix(core): 开了模糊音时 tinwosuo 按 ting 读，不再被纠成请我说" -m "上游 #319：tin 不在音节表，切成 ti n… 后非末尾残缺把整段纠错放了进来，换字母不限邻键，t→q 读成 请我说。
引擎在 set_fuzzy 时建模糊源拼写表，查询、纠错、上屏对照、联想、按音节删除与移光标都用同一张表切分；
最优切分里有模糊音节仍算不像话，整段纠错照样试，但原样分已是模糊读法，纠正要高出一次编辑代价才接管（dakaiho 仍纠成 打开后，rinshi 开 l-r 出 临时）。
模糊音全关时表为 None，所有路径同改前。"
```

---

### Task 4: 双拼解码用同一张表（上游 #306 / PR #330 的用例）

**Files:**
- Modify: `crates/qingjian-core/src/shuangpin/scheme.rs:171-215`（`decode_with`、`fuzzy_syllable`）
- Modify: `crates/qingjian-core/src/engine/setup.rs:160-164,204-209`（`takes_semicolon` / `decode` 传表）
- Modify: `crates/qingjian-core/tests/fuzzy_engine.rs`（加双拼测试）
- Create: `crates/qingjian-core/tests/shuangpin_fuzzy.rs`（PR #330 原样）

- [ ] **Step 1: 写失败的测试**

(a) `crates/qingjian-core/tests/fuzzy_engine.rs` 改两处、末尾加一个测试：

第 1 行文件头替换为：

```rust
//! 模糊源拼写在引擎里：查询、整段纠错让位、双拼解码、上屏消耗、按音节移动删除（上游 #319、#306）。
```

第 3 行导入替换为：

```rust
use qingjian_core::{Engine, FuzzyRules, ShuangpinScheme};
```

文件末尾追加：

```rust
#[test]
fn shuangpin_pairs_decode_through_the_same_sources() {
    let mut engine = engine(DICT);
    engine.set_shuangpin(Some(ShuangpinScheme::Ziranma));
    engine.set_fuzzy(rules(&["s-sh"]));
    assert_eq!(first(&mut engine, "sd"), "双");
    assert_eq!(first(&mut engine, "sdrf"), "双人");
    engine.set_fuzzy(FuzzyRules::default());
    assert!(!offers(&mut engine, "sd", "双"));
    // 小鹤 tb = t + in：开 in/ing 后同一条路
    engine.set_shuangpin(Some(ShuangpinScheme::Xiaohe));
    engine.set_fuzzy(rules(&["in-ing"]));
    assert_eq!(first(&mut engine, "tb"), "听");
    assert_eq!(first(&mut engine, "tbwouo"), "听我说");
}
```

(b) 新建 `crates/qingjian-core/tests/shuangpin_fuzzy.rs`（上游 PR #330 的三个测试，一字不改）：

```rust
//! 双拼模糊音的查询与上屏回归。

use qingjian_core::{Engine, FuzzyRules, ShuangpinScheme};
use qingjian_dictionary::Dictionary;

#[test]
fn fuzzy_shuangpin_accepts_suang_and_consumes_only_its_keys() {
    for scheme in ShuangpinScheme::ALL {
        let dictionary =
            Dictionary::parse("双\tshuang\t9000\n人\tren\t8000\n双人\tshuang ren\t12000\n")
                .unwrap();
        let mut engine = Engine::new(dictionary);
        engine.set_shuangpin(Some(scheme));
        engine.set_fuzzy(FuzzyRules {
            s_sh: true,
            ..FuzzyRules::default()
        });
        let keys = format!("s{}", scheme.encode("shuang").unwrap()[1]);
        let remaining: String = scheme.encode("ren").unwrap().iter().collect();
        engine.set_input(&keys);
        let query = engine.query().expect("s/sh 模糊音应让对应的双拼键查询到双");
        let candidate = query
            .candidates
            .items
            .iter()
            .find(|c| c.text == "双")
            .expect("缺少双")
            .clone();
        assert_eq!(engine.commit(&candidate), "双");
        assert!(engine.composition().is_empty());
        for separator in ["", "'"] {
            engine.set_input(&format!("{keys}{separator}{remaining}"));
            let query = engine.query().unwrap();
            assert!(
                query.candidates.items.iter().any(|c| c.text == "双人"),
                "{scheme}"
            );
            let candidate = query
                .candidates
                .items
                .iter()
                .find(|c| c.text == "双")
                .unwrap()
                .clone();
            assert_eq!(engine.commit(&candidate), "双");
            assert_eq!(engine.composition().text(), remaining, "{scheme}");
        }
    }
}

#[test]
fn fuzzy_pair_keeps_semicolon_final_available_for_the_next_syllable() {
    for scheme in [ShuangpinScheme::Microsoft, ShuangpinScheme::Sogou] {
        let dictionary = Dictionary::parse("双星\tshuang xing\t9000\n").unwrap();
        let mut engine = Engine::new(dictionary);
        engine.set_shuangpin(Some(scheme));
        engine.set_fuzzy(FuzzyRules {
            s_sh: true,
            ..FuzzyRules::default()
        });
        engine.set_input("sdx");
        assert!(engine.takes_semicolon());
        engine.push(';');
        let query = engine.query().unwrap();
        let candidate = query
            .candidates
            .items
            .iter()
            .find(|c| c.text == "双星")
            .unwrap();
        assert_eq!(engine.commit(candidate), "双星");
        assert!(engine.composition().is_empty());
    }
}

#[test]
fn fuzzy_toggle_does_not_admit_invalid_pairs_or_change_normal_decoding() {
    let dictionary =
        Dictionary::parse("双\tshuang\t9000\n三\tsan\t8000\n思\tsi\t7000\n是\tshi\t6000\n")
            .unwrap();
    let mut engine = Engine::new(dictionary);
    engine.set_shuangpin(Some(ShuangpinScheme::Ziranma));
    for enabled in [false, true, false, true] {
        engine.set_fuzzy(FuzzyRules {
            s_sh: enabled,
            ..FuzzyRules::default()
        });
        engine.set_input("sd");
        let found = engine
            .query()
            .is_ok_and(|q| q.candidates.items.iter().any(|c| c.text == "双"));
        assert_eq!(found, enabled);
        engine.set_input("sj");
        assert_eq!(engine.query().unwrap().candidates.items[0].text, "三");
        engine.set_input("si");
        let query = engine.query().unwrap();
        assert_eq!(query.candidates.items[0].text, "思");
        assert_eq!(
            query.candidates.items.iter().any(|c| c.text == "是"),
            enabled
        );
        engine.set_input("bd");
        assert!(engine.query().is_err());
    }
    engine.set_fuzzy(FuzzyRules {
        n_l: true,
        ..FuzzyRules::default()
    });
    engine.set_input("sd");
    assert!(engine.query().is_err());
}
```

- [ ] **Step 2: 跑测试，确认失败**

Run: `cargo test -p qingjian-core --no-fail-fast --test fuzzy_engine --test shuangpin_fuzzy`
Expected: `fuzzy_engine` 7 passed / 1 failed（`shuangpin_pairs_decode_through_the_same_sources`：自然码 `sd` 解码停在尾巴、没有 双）；`shuangpin_fuzzy` 3 个全失败。

- [ ] **Step 3: 实现**

`Scheme::decode_with(keys, fuzzy)`：两键配不出合法音节时，`fuzzy_syllable` 取「该键的声母 + 第二键的任一韵母」落在模糊源表里的原拼写（`s` + `uang` = `suang`），之后照常模糊展开成 shuang；`decode` = `decode_with(keys, None)`，所以没开模糊音、或开的规则和这对键无关（n-l）时 `sd` 仍报错。`Engine::decode` 与 `takes_semicolon` 传 `self.fuzzy_sources.as_ref()`（`;` 续输时落单声母的判断同一张表）。

```bash
cat > /tmp/d1-t4.patch <<'PATCH'
diff --git a/crates/qingjian-core/src/engine/setup.rs b/crates/qingjian-core/src/engine/setup.rs
index 7a7fb2e7..0470aec9 100644
--- a/crates/qingjian-core/src/engine/setup.rs
+++ b/crates/qingjian-core/src/engine/setup.rs
@@ -160,7 +160,11 @@ impl Engine {
             .question_body(self.composition.scope(), self.zhuyin);
         self.shuangpin
             .filter(|scheme| scheme.uses_semicolon())
-            .is_some_and(|scheme| scheme.decode(body).pending_initial())
+            .is_some_and(|scheme| {
+                scheme
+                    .decode_with(body, self.fuzzy_sources.as_ref())
+                    .pending_initial()
+            })
     }
 
     /// 只用形码：码表挂着、拼音侧关着。
@@ -204,8 +208,9 @@ impl Engine {
         if self.zhuyin {
             Some(EngineDecoded::Zhuyin(crate::zhuyin::decode(keys)))
         } else {
-            self.shuangpin
-                .map(|scheme| EngineDecoded::Shuangpin(scheme.decode(keys)))
+            self.shuangpin.map(|scheme| {
+                EngineDecoded::Shuangpin(scheme.decode_with(keys, self.fuzzy_sources.as_ref()))
+            })
         }
     }
 
diff --git a/crates/qingjian-core/src/shuangpin/scheme.rs b/crates/qingjian-core/src/shuangpin/scheme.rs
index 0480f08e..5c29e0c4 100644
--- a/crates/qingjian-core/src/shuangpin/scheme.rs
+++ b/crates/qingjian-core/src/shuangpin/scheme.rs
@@ -171,6 +171,12 @@ impl Scheme {
     /// 把敲的键翻成全拼。两键一组从左到右配对；配不出合法音节的位置起全部原样留作尾巴；
     /// 末尾落单的一键当声母（或元音）前缀；用户自己敲的 `'` 结束当前配对。
     pub fn decode(self, keys: &str) -> Decoded {
+        self.decode_with(keys, None)
+    }
+
+    /// 同 [`Self::decode`]，`fuzzy` 是只有模糊音才读得通的拼写（[`crate::FuzzyRules::sources`]）：
+    /// 两键配不出合法音节时，声母 + 韵母落在里面也收（`s` + `uang` = `suang`，开着 s/sh 时查 shuang）。
+    pub fn decode_with(self, keys: &str, fuzzy: Option<&parser::SyllableTrie>) -> Decoded {
         let chars: Vec<char> = keys.chars().collect();
         let mut units = Vec::with_capacity(chars.len() / 2 + 1);
         let mut index = 0;
@@ -183,11 +189,14 @@ impl Scheme {
             }
             let second = chars.get(index + 1).copied().filter(|c| *c != '\'');
             let unit = match second {
-                Some(second) => self.syllable(first, second).map(|pinyin| Unit {
-                    keys: [first, second].iter().collect(),
-                    pinyin,
-                    complete: true,
-                }),
+                Some(second) => self
+                    .syllable(first, second)
+                    .or_else(|| self.fuzzy_syllable(first, second, fuzzy?))
+                    .map(|pinyin| Unit {
+                        keys: [first, second].iter().collect(),
+                        pinyin,
+                        complete: true,
+                    }),
                 None => self.partial(first).map(|pinyin| Unit {
                     keys: first.to_string(),
                     pinyin,
@@ -203,6 +212,20 @@ impl Scheme {
         Decoded::new(units, chars[index..].iter().collect())
     }
 
+    /// 两键的声母 + 韵母只有模糊音才读得通时的原拼写（`suang`）。
+    fn fuzzy_syllable(
+        self,
+        first: char,
+        second: char,
+        fuzzy: &parser::SyllableTrie,
+    ) -> Option<String> {
+        let initial = self.initial(first)?;
+        self.finals(second)
+            .iter()
+            .map(|final_| format!("{initial}{final_}"))
+            .find(|raw| fuzzy.contains(raw))
+    }
+
     /// 落单的一键代表的前缀：声母键是声母，元音键是元音本身（`a` 后面可能是 ai / an / ang / ao）。
     fn partial(self, key: char) -> Option<String> {
         if let Some(initial) = self.initial(key) {
PATCH
git apply --check /tmp/d1-t4.patch && git apply /tmp/d1-t4.patch
```

- [ ] **Step 4: 跑测试，确认通过**

Run: `cargo test -p qingjian-core`
Expected: 0 failed；`fuzzy_engine` 8 passed、`shuangpin_fuzzy` 3 passed、`shuangpin_umlaut` 照旧全过。

- [ ] **Step 5: 提交**

```bash
cargo fmt --all && cargo clippy -p qingjian-core --all-targets -- -D warnings
git add crates/qingjian-core/src/shuangpin crates/qingjian-core/src/engine/setup.rs crates/qingjian-core/tests/fuzzy_engine.rs crates/qingjian-core/tests/shuangpin_fuzzy.rs
git commit -m "fix(core): 双拼开 s/sh 后 suang 的键也能出双" -m "上游 #306：Scheme::syllable 只认合法的声母 + 韵母，s + uang 不合法，解码停在这里留作尾巴，模糊音没机会作用。
上游 PR #330 在 decode 里对每对键现场跑一次 expand；这里改用 set_fuzzy 时算好的模糊源拼写表，与全拼切分是同一个机制（小鹤 tb 开 in-ing 也出 听）。
tests/shuangpin_fuzzy.rs 是 #330 的三个测试，原样收进来。"
```

---

### Task 5: 回归验收（不提交代码，只跑数）

**Files:** 无改动

- [ ] **Step 1: 全量检查**

```bash
cd /Users/liyuqing/sproot/qingjian-fuzzy
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test -p qingjian-core
(cd cloud && cargo check --all-targets)
```

Expected: 全部成功、0 warning。`cloud/` 不改代码，只确认依赖 `qingjian-core` 的桥还编得过（`Syllable` 加了公开字段、`parser` 多了公开项）。

- [ ] **Step 2: 编改后的 CLI，跑三种模糊配置**

```bash
cargo build --release -p qingjian-cli
/tmp/d1-eval/run.sh "$PWD/target/release/qingjian-cli" new-off
/tmp/d1-eval/run.sh "$PWD/target/release/qingjian-cli" new-in-ing_s-sh in-ing,s-sh
/tmp/d1-eval/run.sh "$PWD/target/release/qingjian-cli" new-all all
```

- [ ] **Step 3: 比较改前改后**

```bash
cd /tmp/d1-eval
for c in off in-ing_s-sh all; do
  echo "== $c"
  diff <(grep -v ' ms' eval-base-$c.txt) <(grep -v ' ms' eval-new-$c.txt)
  diff <(grep -v ' ms' replay-base-$c.txt) <(grep -v ' ms' replay-new-$c.txt)
  cmp details-base-$c.jsonl details-new-$c.jsonl && echo "details 逐条一致"
done
```

（`grep -v ' ms'` 只去掉耗时行：`查询平均 … ms`、`平均翻页 … / 耗时 … ms`、`按键同步部分 p50 … ms`，它们每次跑都不同。）

Expected，写计划时实测：

```text
== off
details 逐条一致
== in-ing_s-sh
details 逐条一致
== all
9c9
<        当时纠错生效 2 条，现在仍纠 2 条
---
>        当时纠错生效 2 条，现在仍纠 1 条
details 逐条一致
```

判据：
- `off` 与 `in-ing_s-sh`：三项 diff 全空。
- `all`：`eval` diff 空、`details` 逐条一致；`replay` 汇总**只允许**上面这一行（英文组，日志第 388 行 `serv` 不再被纠成 `se qv`，原因见「与规格的差异」第 3 条）。核对它确实是 `serv`：

```bash
cd /Users/liyuqing/sproot/qingjian-fuzzy
/tmp/d1-base/qingjian-cli --config tools/eval/offline.toml --fuzzy all serv 2>/dev/null | head -4
target/release/qingjian-cli --config tools/eval/offline.toml --fuzzy all serv 2>/dev/null | head -4
```

Expected: 改前有 `纠正: serv → se qv`，改后切分 `se rv | s… e rv`、没有纠正行，两边首选都是 `service`。
- 出现任何别的差异：**停下**，不要提 PR；用 `--replay-details` 的行号定位、把差异写进汇报（必要时转 Bug修复大师）。

- [ ] **Step 4: 改后的 gate-pass，与改前逐位比**

```bash
cd /Users/liyuqing/sproot/qingjian-fuzzy
tools/release/gate-pass.sh; echo "exit=$?"
for s in none fusion; do
  diff <(grep -v ' ms' /tmp/d1-base/gate-eval-$s.txt) <(grep -v ' ms' /tmp/gate-eval-$s.txt)
  diff /tmp/d1-base/gate-replay-$s.txt /tmp/gate-replay-$s.txt
done && echo "gate-pass 改前改后逐位一致"
```

Expected: 打印 `gate-pass 改前改后逐位一致`（gate-pass 用缺省配置 = 模糊音全关，`fuzzy_sources` 为 `None`，所有路径同改前）；`exit` 与 Task 0 Step 6 相同。

- [ ] **Step 5: 定点看效果（规格 §4.3 的表）**

```bash
cd /Users/liyuqing/sproot/qingjian-fuzzy
NEW=target/release/qingjian-cli; OLD=/tmp/d1-base/qingjian-cli
for args in "--fuzzy in-ing,s-sh tinwosuo" "--fuzzy s-sh suang" "--fuzzy f-h fui" "--fuzzy c-ch cuangkou" \
            "--fuzzy l-r rinshi" "--shuangpin ziranma --fuzzy s-sh sd" "--shuangpin xiaohe --fuzzy in-ing tbwouo" "tinwosuo"; do
  for c in $OLD $NEW; do
    printf '%s | %s: ' "$(basename "$(dirname "$c")")" "$args"
    $c --config tools/eval/offline.toml $args 2>/dev/null | grep -E '^ +1\. |纠正' | tr '\n' ' '; echo
  done
done
```

（在 zsh 里跑要把 `$args` 写成 `${=args}` 才会按空格拆参数；bash 下原样即可。）

Expected（首选，改前 → 改后）：`tinwosuo`（in-ing,s-sh）纠正成 请我说 → **听我说**、无纠正；`suang` 数羊 → **双**；`fui` 服 → **会**；`cuangkou` 出港口 → **窗口**；`rinshi`（l-r）纠正成 认识 → **临时**；自然码 `sd` SD → **双**；小鹤 `tbwouo` 无候选 → **听我说**；`tinwosuo`（全关）替你我所 → 替你我所（不变）。把这张表贴进 PR 描述。

---

### Task 6: 文档与分叉补丁记录

**Files:**
- Modify: `docs/user/input/fuzzy-and-shuangpin.md:12`
- Modify: `docs/notes/crate-notes.md:61`（「拼写纠错（`correction`）」那条之前插一条）
- Modify: `cloud/docs/fork-patch.md`（「## 合并上游时」之前加一节）

- [ ] **Step 1: 用户文档**

`docs/user/input/fuzzy-and-shuangpin.md` 里找到这一行：

```markdown
输入正确的词仍然优先，模糊匹配的词排在其后。韵母三条只对完整音节生效。
```

在它后面空一行，插入：

```markdown
不合法的拼写也能走模糊音：开启 in/ing 后输入 `tinwosuo` 出 听我说（`tin` 本身不是拼音，按 ting 读），开启 s/sh 后输入 `suang` 出 双，开启 f/h 后输入 `fui` 出 会；
双拼同理，自然码开启 s/sh 后输入 `sd` 出 双。只有对应那条模糊音开着时才这样读；输入的本来就是合法拼音时，候选与以前一样。
改一个字母就能读成通顺得多的句子时，仍按拼写纠错处理（全部开启时输入 `dakaiho` 仍出 打开后）。
```

- [ ] **Step 2: crate-notes**

`docs/notes/crate-notes.md` 里找到以这句开头的那条：

```markdown
- 拼写纠错（`correction`）：整段一处编辑的变体里相邻换位允许末尾音节没敲完（`loose_segmentation`，`mignt` → `ming t…`；
```

在它**前面**插入一条：

```markdown
- 模糊源拼写（`fuzzy/sources.rs`，上游 #319 / #306，规格 `docs/notes/wetype-impl/d1-fuzzy-as-augmenter.md`）：`FuzzyRules::sources` 枚举「本身不合法、套一次开着的规则就合法」的拼写（全开 60 个），
  `Engine::set_fuzzy` 时建成小 `SyllableTrie`（`Engine::fuzzy_sources`，全关为 `None`，所以全关时一切同改前）。`parser::segment_with` 让切分把它们也当完整音节（`Syllable::fuzzy`），之后照常经 `FuzzyRules::expand` 查词；
  `segment_longest_prefix` 的所有调用（查询、整段纠错、上屏对照、联想、`marked_rest`、按音节删除 / 移光标）都传这张表；双拼 `Scheme::decode_with` 两键配不出合法音节时用同一张表（`s` + `uang`）。
  两条护栏：排序第二键是「残缺 + 模糊」个数（`shoufe` 仍是 `shou fe…`）；最优切分不含模糊音节时整段退回不带表的切分（`dingan` 的候选与不开模糊音逐条一致，代价是 `dinan` 不出 定案）。
  最优切分有模糊音节也算 `unlikely_pinyin`：整段纠错照样试，但原样分已是模糊读法（含每音节 ln 2），纠正要高出一次编辑代价才接管（`tinwosuo` 不再被纠成 请我说，`dakaiho` 仍纠成 打开后）。
  回放（2026-10-11）：全关 / in-ing,s-sh / 全开下 `--eval-text` 与 clean2 `--replay-details` 逐条不变；全开时英文组「现在仍纠」2 → 1（`serv` 不再被纠成 `se qv`，改按模糊读 `se rv`，前五不变）。
  没做：整段纠错「换一个字母」不限邻键（#319 第二层根因，另立项）。
```

- [ ] **Step 3: fork-patch.md**

`cloud/docs/fork-patch.md` 里找到 `## 合并上游时`，在它**前面**插入：

```markdown
### 模糊源拼写当完整音节（Core 上游文件，各平台都生效）

上游 #319（开 in-ing、s-sh 后 `tinwosuo` 被纠成 请我说）、#306（双拼开 s-sh 后 `suang` 的键报错）。根因是切分只认合法音节，`tin` / `suang` 这类「套一次模糊规则才合法」的拼写模糊音根本没参与。
设计与回放见 `docs/notes/wetype-impl/d1-fuzzy-as-augmenter.md` 与 `docs/notes/crate-notes.md`「模糊源拼写」条。

| 文件 | 改动 | 说明 |
|---|---|---|
| `crates/qingjian-core/src/fuzzy/sources.rs` | 新文件 | `FuzzyRules::sources` |
| `crates/qingjian-core/src/fuzzy/mod.rs` | 加 1 行 | `mod sources;` |
| `crates/qingjian-core/src/fuzzy/rules.rs` | 拆 1 个函数 | `apply_once` 拆出不过滤的 `swaps`，两者 `pub(super)`；行为不变 |
| `crates/qingjian-core/src/parser/segmentation.rs` | 加约 20 行 | `Syllable.fuzzy`、`Syllable::fuzzy`、`Segmentation::fuzzy_count` |
| `crates/qingjian-core/src/parser/mod.rs` | 改约 50 行 | `segment_with`（`segment` = 不带表）、DP 收模糊 trie、`sort_key` 第二键、合法优先护栏 |
| `crates/qingjian-core/src/engine/mod.rs` | 加约 10 行 | `fuzzy_sources` 字段；`segment_longest_prefix` / `marked_rest` 多一个参数 |
| `crates/qingjian-core/src/engine/setup.rs` | 加约 10 行 | `set_fuzzy` 建表；`decode` / `takes_semicolon` / `marked_rest` 传表 |
| `crates/qingjian-core/src/engine/composing.rs` | 改约 30 行 | `unit_len_before` / `unit_len_after` 多一个参数（按音节删除、移光标） |
| `crates/qingjian-core/src/engine/{query/phonetic.rs,correcting.rs,commit/mod.rs,prediction/mod.rs}` | 各改 1–2 处调用 | 传表；`prediction/mod.rs` 的存量 `use super::*` 没顺手换，免得冲突面变大 |
| `crates/qingjian-core/src/correction/mod.rs` | 改 1 个函数 | `unlikely_pinyin`：模糊音节也算不像话 |
| `crates/qingjian-core/src/shuangpin/scheme.rs` | 加约 25 行 | `decode_with` + `fuzzy_syllable`；`decode` = 不带表 |
| `crates/qingjian-core/tests/{fuzzy_sources,fuzzy_engine,shuangpin_fuzzy}.rs` | 新文件 | `shuangpin_fuzzy.rs` 是上游 PR #330 的测试原样 |
| `docs/user/input/fuzzy-and-shuangpin.md`、`docs/notes/crate-notes.md` | 加段 | 用户说明、实现要点 |

合并上游时：**上游若先合了 PR #330**，`shuangpin/scheme.rs` 与 `engine/setup.rs` 会冲突——保留本补丁的 `decode_with`（它覆盖 #330 的全部用例，#330 的测试已在 `tests/shuangpin_fuzzy.rs`），删掉 #330 在 `decode` 里现场跑 `expand` 的那段，再跑 `cargo test -p qingjian-core --test shuangpin_fuzzy --test fuzzy_engine`。
上游若自己修了 #319，对照它与本补丁的切分结果，用 Task 5 的三配置回放比，取不伤合法输入的那个。
```

- [ ] **Step 4: 提交**

```bash
cd /Users/liyuqing/sproot/qingjian-fuzzy
git add docs/user/input/fuzzy-and-shuangpin.md docs/notes/crate-notes.md cloud/docs/fork-patch.md
git commit -m "docs(core): 模糊音也读不合法的拼写，记实现要点与分叉补丁" -m "用户能感知：开了模糊音后 tinwosuo、suang、双拼 sd 的结果变了，用户页写明「不合法的拼写也能走模糊音」与纠错仍会接管的情形。
crate-notes 记模糊源拼写表、两条护栏、纠错比较口径与全开回放那一行已知差异；fork-patch.md 记改了哪些上游文件、与 #330 冲突时怎么合。"
```

---

### Task 7: 推分支、开 PR（不合并）

**Files:** 无

- [ ] **Step 1: 推送**

```bash
cd /Users/liyuqing/sproot/qingjian-fuzzy
git remote -v   # 确认 origin 是 https://github.com/dayuer/qingjian.git；upstream 的 push 是 DISABLED
git push -u origin fuzzy/sources
```

Expected: pre-push 钩子跑全 workspace 测试通过、推送成功。

- [ ] **Step 2: 开 PR**

把 Task 5 Step 3 与 Step 5 的实际输出替换进下面「怎么验证的」里的两段（数字与表照实填）：

```bash
gh pr create --repo dayuer/qingjian --base sujian --head fuzzy/sources \
  --title "fix(core): 模糊源拼写当完整音节（tinwosuo 听我说、双拼 suang 双）" \
  --body "$(cat <<'BODY'
## 改了什么

修上游 #319（开 in-ing、s-sh 后 `tinwosuo` 被纠成 请我说）与 #306（双拼开 s-sh 后 `suang` 的键报错）。
切分只认合法音节，`tin` / `suang` 这类「套一次开着的模糊规则才合法」的拼写模糊音根本没参与。现在 `set_fuzzy` 时算出这些拼写（全开 60 个），切分与双拼解码把它们当完整音节；
最优切分有这种音节时整段纠错照样试，但要比模糊读法高出一次编辑代价才接管。模糊音全关时一切同改前。上游 PR #330 的三个测试原样收进 `tests/shuangpin_fuzzy.rs` 并通过。
不做：整段纠错「换一个字母」不限邻键（#319 第二层根因），另立项。

## 平台

- [x] 与平台无关（Core / 数据 / 文档）

## 合并前清单

- [x] `cargo fmt --all --check`、`cargo clippy --all-targets -- -D warnings`、`cargo test -p qingjian-core`、`cloud/` 下 `cargo check --all-targets` 本机全过
- [x] 用户能感知的行为变了，`docs/user/input/fuzzy-and-shuangpin.md` 已同步；按键没变，`keys.md` 不动
- [x] 新文件有 `//!` 头，单文件不超过 800 行
- [x] 改了上游文件，`cloud/docs/fork-patch.md` 已记

## 怎么验证的

Core 逻辑，有集成测试与回放兜底（contributing「外部 PR」一节：这类改动不要求真机）。

1. 定点（`qingjian-cli --config tools/eval/offline.toml`，改前 → 改后首选）：
   （贴 Task 5 Step 5 的表）
2. 回放与评测，三种模糊配置（全关 / in-ing,s-sh / 全开），`--eval-text data/eval/sentences.tsv` 与 `--replay data/eval/input-log-2026-10-04.jsonl --clean2 --replay-details`：
   （贴 Task 5 Step 3 的输出）
   全开时汇总只有一行不同：英文组「现在仍纠」2 → 1，是日志第 388 行 `serv` 不再被纠成无意义的 `se qv`（改按模糊读 `se rv`），首选与前五不变；`--replay-details` 逐条一致。
3. `tools/release/gate-pass.sh`（缺省 = 模糊音全关）改前改后输出逐位一致。
4. 评测集与回放日志都是模糊音关着时采的，几乎不含 `tin` 这类拼写：上面只证明不伤合法输入，收益靠 `tests/fuzzy_engine.rs` 的定点测试。

🤖 Generated with [Claude Code](https://claude.com/claude-code)
BODY
)"
```

Expected: 打印 PR 链接（`https://github.com/dayuer/qingjian/pull/<N>`）。**不合并**，把链接交回给用户。

- [ ] **Step 3: 收尾提醒**

PR 由用户明确说「合 #N」后才合；合了以后：

```bash
git -C /Users/liyuqing/sproot/qingjian worktree remove --force ../qingjian-fuzzy
git -C /Users/liyuqing/sproot/qingjian branch -D fuzzy/sources
```

---

## Self-Review

- **规格覆盖（§4.1 步骤 1–7）：** 1 → Task 1；2 → Task 2；3（字段、`set_fuzzy`、全部调用方含 `composing.rs` / `prediction` / `marked_rest`）→ Task 3；4 → Task 3；5 → Task 4；6 → Task 6；7 → Task 0 + Task 5。
- **§4.2 单测清单逐条：** `sources_are_the_illegal_spellings_one_rule_away`、`sources_match_the_audited_table_rule_by_rule`（§3.2 表）→ `fuzzy_sources.rs`（Task 1）；`segmentation_reads_fuzzy_sources_as_complete_but_after_legal_ones`（含 `dingan`/`dinan`/`nihao` 逐条相等、`shoufe`）、`fuzzy_syllables_stay_inside_their_apostrophe_chunk`（`tin'wo`）、`legal_segmentation_survives_truncation_when_the_best_is_fuzzy`（`MAX_SEGMENTATIONS`）→ Task 2；
  `tinwosuo_is_read_by_fuzzy_rules_not_corrected`（含全关对照、`dingan` 定案）、`suang_needs_s_sh`、`committing_a_fuzzy_word_consumes_its_typed_keys`、`syllable_moves_and_deletes_treat_a_fuzzy_spelling_as_one_syllable`（`composing.rs`）、`a_better_correction_still_wins_over_a_fuzzy_reading`（`dakaiho`）、`a_fuzzy_reading_beats_a_one_edit_correction`（`rinshi`）、`english_words_keep_their_place_with_every_rule_on`（`hone`/`serv`）→ Task 3；`shuangpin_pairs_decode_through_the_same_sources` 与 PR #330 三个 → Task 4。
- **验收：** gate-pass 逐位 → Task 5 Step 4；两种模糊配置 eval / replay / details diff → Task 5 Step 3（全开那一行已知差异写明来由与核对命令）；用户页「不合法的拼写也能走模糊音」→ Task 6 Step 1；fork-patch → Task 6 Step 3。
- **类型一致：** `FuzzyRules::sources() -> Vec<String>`、`parser::SyllableTrie::build(&[&str])` / `contains(&str)` / `matches(&str).lengths()`、`parser::segment_with(&str, Option<&SyllableTrie>)`、`segment_longest_prefix(&str, Option<&parser::SyllableTrie>)`、`Scheme::decode_with(self, &str, Option<&parser::SyllableTrie>)`、`Syllable::fuzzy(&str)`、`Segmentation::fuzzy_count()`——各补丁与测试里用法一致，已在临时工作树逐任务编译通过。
