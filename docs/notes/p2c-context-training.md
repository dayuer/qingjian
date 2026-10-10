# 通变看上文：带前文重训（2026-10-10）

> **状态：实验，未过验收。不要合、不要开 PR。**
> 第三轮没过硬线（回放 clean2 整句 103/138 → 93/138），停在原地等下一轮方案被批。
>
> **范围**：下面「推理侧接前文」那节动了 Core 侧与三个壳的模型预热自检
> （`crates/qingjian-neural`、`apps/cli`、`apps/{macos,linux,windows}/*/rescore`），
> **不在本轮审的范围内**，Core 集成另行派活——要合必须先过那一道审。

给 `train_p2c.py` 加 `--context`：序列改成 `<eos> [上文 ≤32 字] 拼音 <sep> 汉字 <eos>`，同一批
字表与结构（8 层 / 448 / 8 头 / context 128），从含章·通变热启动、lr 3e-5。数据由
`tools/neural-train/context_data.py` 造，三种上文各占三分之一：

| 类型 | 上文 | 对应场景 |
| --- | --- | --- |
| 空 | （无） | 从句首起打，保住原有能力 |
| 切开 | 同一句按词边界切开的前段 | 分段上屏，接着打后半 |
| 上一句 | 同篇文档的上一句（末 32 字） | 换行起的下一句 |

每类里 LCCC 对话与维基书面各占 50%（`--per-kind 400000` → 每类 40 万，共 120 万）。
目标句**与上文**都不许命中三份冻结留出集与派生 dev 集（逐句精确匹配），命中即剔。
停线：书面干净子集比基线 36.1 低 3 个点以上就停；验收看两份空上文都不掉 0.5 点以上、有上文那份要涨。

## 2026-10-10 连崩两次：字表成员检查漏了目标句与上文

**根因**：`usable()` 查的是「逐字有读音」（`readings`），而读音表比 8180 的 token 字表宽。
`皝` 这类罕见字有读音、没进字表，于是 `build_stream` 里 `vocab[ch]` 抛 `KeyError`——
第一次崩在上文（`previous_text[-32:]` / 切开前段），第二次崩在目标句本身。
两处都只查了拼音侧，没查汉字侧。

**新检查**：`context_data.in_vocab(text, vocab)` 统一判定「整串都在字表里」，三条取样分支
（空 / 切开 / 上一句）的目标、上文、切尾拼音全部过它；`build_stream` 再加一层兜底——
字表外字符直接跳过并计数打印，不再让一条脏句子杀掉十分钟的数据构建。
样本缓存键升 `v=3`（旧缓存不含该过滤，必须作废）。

**作废结果**：前三次启动全部作废（两次 `KeyError: '皝'`、一次把 `--layers` 打成 `--laysers` 被
argparse 打回）。本次启动的训练集 sha256 `7944ae1a7a5ecba0`，1,177,190 条 / 剔 22,810（1.90%），
流 47,004,727 token，字表外跳过 0。**只有 sha 为 `7944ae1a7a5ecba0` 的这一轮算数。**

**为什么 sha 与崩前那版不同**（`0e006c0fa9cf80de`）：保留条数一样（配额本来就填满 120 万），
但字表外的句子不再占配额，由后面的句子补进，内容变了。

## 2026-10-10 第四次错：改完没重编就量，把旧二进制的结果当真值

**根因**：给推理侧加「前文开关」（下面那节）时只跑了 `cargo test`（debug 构建），
没重编 `target/release/qingjian-cli`；验收脚本 `accept.py` / `accept_ctx.py` 只调用现成的二进制，不会替我编。
于是那一轮量到的是**加门之前**的二进制：通变被喂了它读不懂的前文，回放 clean2 整句 99/138，
我拿它当了基线，还据此去查「改动是不是动到了通变」。

**新检查**：`accept_ctx.py` 开跑第一件事就是打 CLI 二进制的 sha256 前 16 位与 mtime；
神经侧一改，先 `cargo build --release -p qingjian-cli` 再量。带门的正确值：通变 103/138，
与改造前的历史日志 103/52/51 逐字吻合。

**作废结果**：`.lab/accept-ctx.log` 整份作废（其中「通变 99/138」是旧二进制）。

## 推理侧接前文（2026-10-10）

`P2cScorer::score` 过去把 `context` 丢掉（`_context`），前文只在 Core 里对整句首词起作用。改法：

- `CharScorer::score_p2c(context, keys, texts)` 与 `P2c::convert(context, keys, …)` 前缀变成 `<eos> [前文] 拼音 <sep>`，与训练格式一致；
- `Vocab::encode_context(context, 上限)`：**字表外的字符（标点、罕见字）直接丢**（训练时上文全是表内字），再取末 `上限` 字；
- **开关是模型自己的属性**：`config.json` 里的 `context_chars`（训练脚本 `--context` 时写出）就是上限，没有这个字段（老通变）一律不喂。
  判定在 `CharScorer::context_chars()`，`score_p2c` 与 `P2c::convert` 内部各判一次——多喂一次就是产品里静默掉分。
  只改接线不加这道门，线上那份通变会开始收到前文而掉分。

## 第三轮结果（2026-10-10，**没过硬线**）

二进制 `5f506ee2`；随包词图 dict `7fafe1b8` / lm `f9fb7b44`；通变 `9878e7ac`、带上文 `61a8698e`（暖启动 + 1 万步、lr 3e-5、三类上文各 40 万 + 每类对话:书面 1:1）。

**融合首选**（两行都开着 Core 的「整句首词接前文」，差值只归模型）：

| 尺子 | 通变 | 带上文 | Δ |
| --- | --- | --- | --- |
| sentences | 38.8 | 38.9 | +0.1 |
| dialog | 65.3 | 66.1 | +0.8 |
| prose | 54.3 | 55.2 | +0.9 |
| external | 60.3 | 61.0 | +0.7 |

**回放 clean2**：词 919/1022 → 916/1022（−3）；**整句 103/138 → 93/138（−10，前半 52→47、后半 51→46）**。
硬线是「回放整句不可回退」——**这一轮没过**。探针那三条（有上文 70.0 / 空上文 62.3 / 书面干净 36.1）全过，
但探针是贪心小尺子，CLI 的融合与回放才是判定尺。

**归因**：同模型 `--neural-context 0` 回放 = 94/138，所以 −10 里 **9 条是权重漂移**（暖启动后再跑 1 万步、lr 3e-5），
只有 1 条来自真用前文。融合尺子涨、回放整句掉，方向相反——回放日志的「正确答案」有一批是当时按下的旧引擎首选
（见 [neural-rescoring.md](neural-rescoring.md)），对更强的模型天然有偏；但硬线就是硬线，不拿观察放过它。

**下一轮方向**：降学习率 / 少步数（先把权重漂移压下去），或把空上文的占比从 1/3 提上去给底座能力加锚。

## 跑法（路径 2026-10-10 迁到 data/archive 之后）

```bash
R=/Users/liyuqing/sproot/qingjian          # 主检出；worktree 里跑就设 QJ_REPO 指它
A=$R/data/archive                          # 三个 worktree 的 .lab 合并迁到这里
$A/neural-lab/venv/bin/python tools/neural-train/train_p2c.py --context --per-kind 400000 \
  --layers 8 --embd 448 --heads 8 --init-from $A/neural-lab/tongbian \
  --epochs 1 --max-steps 10000 --max-size-mb 60 --device mps --lr 3e-5 \
  --eval-every 2000 --eval-dev tools/neural-train/context-dev.tsv --out $A/neural-lab/ctx-smoke3
```

脚本里的路径一律按**仓库根**解析（`QJ_REPO` / `QJ_ARCHIVE` 可覆盖）。两个文件不入库、按需现生成：

- `context-dev.tsv`（带上文 dev，496 行，里面是留出集的句子）：
  `python3 -c "import sys; sys.path.insert(0,'tools/neural-train'); import context_data as c, train_p2c as t; print(c.derive_context_dev(t.load_readings()))"`
- `.cache-ctx-samples.tsv`（120 万条样本的缓存，75MB，改取样逻辑要升 `v=` 版本号）。
