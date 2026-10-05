# 分叉补丁

Cloud 的大部分代码在 `cloud/` 下，与上游隔离。只有学习数据同步非改输入法不可，这部分是分叉补丁。
原则：新代码放新文件，对上游已有文件只加挂钩行，合并上游时冲突面最小。

## 清单

| 文件 | 改动 | 说明 |
|---|---|---|
| `crates/qingjian-core/src/engine/learning/learner.rs` | 加 5 行 | `Learner::merge_remote`，缺省实现什么都不做 |
| `crates/qingjian-learning/src/frequency_learner/remote.rs` | 新文件 | 收件箱解析与合并（含测试） |
| `crates/qingjian-learning/src/frequency_learner/mod.rs` | 加 1 行 | `mod remote;` |
| `crates/qingjian-learning/src/frequency_learner/learner_impl.rs` | 加 4 行 | `FrequencyLearner` 实现 `merge_remote` |
| `apps/macos/src/host/cloud/inbox.rs` | 新文件 | 每拍读 `sync/inbox.tsv`、合并、删文件 |
| `apps/macos/src/host/cloud/mod.rs` | 加 1 行 | `mod inbox;` |
| `apps/macos/src/host/config/mod.rs` | 加 1 行 | `tick()` 里调 `apply_cloud_inbox()` |

### 回车上屏高亮候选（用户要的小功能，只在 macOS 生效）

| 文件 | 改动 | 说明 |
|---|---|---|
| `crates/qingjian-platform/src/config/general.rs` | 加 4 行 | `[general] enter_commits_candidate`，缺省 false |
| `crates/qingjian-platform/src/config/mod.rs` | 加 2 行 | 配置模板里的说明 |
| `apps/macos/src/imk/controller/command.rs` | 加 18 行 | 回车分支按开关选高亮 / 原样；⇧ 从当前键盘状态读 |
| `apps/macos/src/preferences/setting/mod.rs` | 加 6 行 | `Setting::EnterCommitsCandidate`，tag 取 99 避开上游往后顺排的号 |
| `apps/macos/src/preferences/pages/general.rs` | 加 20 行 | 「通用」页勾选框 |
| `apps/macos/src/host/settings.rs` | 加 4 行 | 勾选框写回配置 |
| `docs/user/getting-started/keys.md` | 改 1 行、加 1 段 | 按键说明 |

### 中文模式下的英文：少抢中文、不学敲错的拼音（Core，各平台都生效）

2026-10-03 真机日志：`gd` → Gd 混进中文句子；回车原样上屏的敲错拼音（`woilaiceshi`）被学成个人英文词，下次就排第一。

| 文件 | 改动 | 说明 |
|---|---|---|
| `crates/qingjian-core/src/engine/extras.rs` | 加约 15 行 | 两字母全大写让中文的规则放宽到「两个字母、带大写」（Gd）；`listed_english` |
| `crates/qingjian-core/src/engine/composing.rs` | 加约 12 行 | 回车原样上屏的串改一处就是两音节以上完整拼音、又不在随包词表里：不学成英文词 |
| `crates/qingjian-core/src/engine/extras.rs`、`mod.rs` | 加约 6 行 | 只有三个字母、没有精确词时，英文补全不排第一（`wod` 我的 → Wodehouse、`ong` → ongoing） |
| `crates/qingjian-core/src/engine/tests/english_fork.rs` | 新文件 | 上面两条的测试 |

试过但没留的（`cli --replay` 与逐词对比验过）：三个字母全大写也让中文——把 GPU / SQL / LLM 一起压下去，词频也分不开（DOA 2760、LLM 2290）；
拼写纠错（含只认相邻换位）读通时英文让中文——`compa` / `claud` 这类英文前缀也会被纠成拼音，压掉上游有意排第一的英文补全。
`doa` → DOA（三个字母的精确词）、`daun` → daunting（四个字母的补全）靠规则分不开，靠清理个人英文词表兜。
三个字母的补全不抢第一的代价：`hel` 只敲三个字母时 help 排第二。回放（两段日志，436 次选词）：中文不变，英文 10/11 → 8/11，少的是当时误上屏的 ongoing 与 Gd。
回放（今天的日志，344 次中文选词）：词 89.6%、整句 71.4% 不变；英文 9/9 → 8/9，少的那条正是当时误上屏的 Gd。

### 敲到声母就出整句（Core，各平台都生效）

用户反馈：别的输入法敲到 `n` 就出「智能」，青简要把 `neng` 打全。上游有意让全拼句子末尾的单个声母不参与整句
（`MIN_PARTIAL_LETTERS = 2`，怕 `woxiangs` 猜错 shuo），整句又固定排第一，于是 `tianqihenh` 首选是「天气很」、`rengongzhin` 的「人工智能」排第 4。
改为 1：末尾单个声母也按前缀参与。

| 文件 | 改动 | 说明 |
|---|---|---|
| `crates/qingjian-core/src/sentence/mod.rs` | 改 1 个常数与注释 | `MIN_PARTIAL_LETTERS` 2 → 1 |
| `crates/qingjian-core/src/sentence/viterbi.rs` | 改 1 个测试 | `wo xiang k…` 读成三个字 |
| `crates/qingjian-core/src/engine/tests/lookup.rs` | 改 1 个断言 | `xiangkaif` → 想开 + f 开头的字 |
| `docs/design/candidate-ui.md` | 改 1 句 | 整句规则 |

评测（2026-10-04，`qingjian-cli --eval-text`，冷启动、无个人数据；评测集用 `docs/user/` 与本机输入日志的文字冻结在 `data/eval/`）：

| | 改前 | 改后 |
|---|---|---|
| 常规整句 1878 句 | 首选 32.7% | 32.7%（全拼不受影响） |
| 末音节截成声母 1863 句 | 首选 3.9%，字准确率 13.0% | 首选 18.3%，字准确率 69.9% |
| 回放本机输入日志：词 / 整句 | 91.4% / 65.6% | 91.4% / 71.9% |
| 查询耗时 | 平均 0.8 ms | 平均 0.9 ms，最慢 5.4 ms |

`woxiangs` 现在是「我想说」，上游担心的猜错没出现。

### 菜单栏不显示「中 / 英」状态项（用户要的小功能，只在 macOS 生效）

它的菜单与系统输入法菜单里「青简」那一组重复。`[general] mode_indicator`，缺省 true（上游行为）；false 时整个状态项不展开。
只有配置项，没加偏好设置的勾选框。

| 文件 | 改动 | 说明 |
|---|---|---|
| `crates/qingjian-platform/src/config/general.rs` | 加 4 行 | 字段与缺省 |
| `crates/qingjian-platform/src/config/mod.rs` | 加 2 行 | 配置模板里的说明 |
| `apps/macos/src/menubar/indicator.rs` | 加 16 行 | `enabled` 字段、`set_enabled`，关着时 `activate` 不展开 |
| `apps/macos/src/host/config/mod.rs` | 加 2 行 | 热加载时按配置开关 |

### 自建更新服务器（只在 macOS 生效）

输入法查 `https://pinyin.synon.ai/releases/releases.json`，只认自己的签名密钥；有新版就在后台下好 pkg 并校验 sha256，
菜单「有新版本」/「关于 → 下载新版」点了打开安装程序（不自动弹，免得打断打字）。

| 文件 | 改动 | 说明 |
|---|---|---|
| `crates/qingjian-update/src/index/signature.rs` | 改 1 行 | `PUBLIC_KEYS` 换成自己的公钥（私钥在发布机 `~/.config/qingjian-cloud/release-signing.key`） |
| `crates/qingjian-update/src/index/fetch.rs` | 改 1 行 | `INDEX_URL` |
| `crates/qingjian-update/src/lib.rs` | 改 1 行、加 2 行 | `DOWNLOAD_URL`、`mod download`、导出 `Package` |
| `crates/qingjian-update/src/download.rs` | 新文件 | 下载到数据目录 `updates/`、校验 sha256、清旧包 |
| `crates/qingjian-update/src/checker/package.rs` | 新文件 | `Package`（文件名、地址、sha256） |
| `crates/qingjian-update/src/index/asset.rs` | 加 3 个字段 | 索引里本来就有的 `file` / `url` / `sha256`，上游没读 |
| `crates/qingjian-update/src/index/mod.rs` | 改 `newest` | 带上本机安装包 |
| `crates/qingjian-update/src/checker/{mod,available,state}.rs` | 加字段与 `downloaded()` | 查到后下载；`UpdateState.downloaded` |
| `crates/qingjian-update/src/error.rs` | 加 1 个变体 | `ChecksumMismatch` |
| `apps/macos/src/host/cloud/update.rs` | 新文件 | `open_update`：有下好的包就打开它，否则开下载页 |
| `apps/macos/src/host/settings.rs` | 改 2 行 | 菜单与「关于」页的按钮改调 `open_update` |
| `apps/macos/src/preferences/pages/about.rs` | 改 2 段文案 | 更新与隐私说明写自建服务器 |
| `apps/macos/scripts/bundle.sh` | 加 2 行 | `QINGJIAN_VERSION` 顶替版本号 |

版本号是 `<上游版本去掉 -dev>-local.<提交数>`，`-dev` 本地包照旧不查更新。发布：`cloud/scripts/publish-mac.sh`，见 `deploy/README.md`。
合并上游时上游改了更新检查，以上游为准重新挂这几处；上游的 `releases.json` 本来就带 `file` / `url` / `sha256`，格式不用动。

### 素笺云链进输入法进程（只在 macOS 生效）

素笺云 的 Mac 端是库 `cloud/crates/qingjian-cloud-mac`，输入法按路径依赖它，不另起常驻程序。它自带 0.5 秒的
NSTimer（输入法自己的定时器失焦就停）与后台线程，入口都包 `catch_unwind`，出错只停同步；每 2 秒看当前输入法，
连续 30 秒不是青简就停掉同步线程，切回来恢复。输入法只调三个函数：`start(mtm)`、`menu_lines()` / `menu_revision()`、`perform(tag)`。

| 文件 | 改动 | 说明 |
|---|---|---|
| `Cargo.toml`（根） | 加 2 行 | `exclude = ["cloud"]`：不然按路径引用 cloud 的 crate 时，它的 `*.workspace = true` 会去找根 workspace |
| `Cargo.lock` | 新增条目 | ureq、native-tls 证书等；`uuid` 1.26.1 → 1.27.0（cloud 要求）；2026-10-04 账号登录新增 objc2-authentication-services（block2、base64、getrandom、percent-encoding、toml_edit 等根 Cargo.lock 里本来就有，只是多了依赖边） |
| `apps/macos/Cargo.toml` | 加 2 行 | 依赖 `qingjian-cloud-mac` |
| `apps/macos/src/main.rs` | 加 2 行 | IMKServer 建好后 `qingjian_cloud_mac::start(mtm)` |
| `apps/macos/src/menubar/cloud_agent.rs` | 新文件 | 照菜单行画「素笺云 ›」子菜单 |
| `apps/macos/src/menubar/action.rs` | 加 1 个变体 | `MenuAction::CloudAgent(tag)`，tag 1000..1200 |
| `apps/macos/src/menubar/menu.rs` | 加字段与 `sync_cloud_agent` | 父项紧挨「模糊音」（见下面 IMK 的坑） |
| `apps/macos/src/menubar/mod.rs` | 加 1 行 | `mod cloud_agent` |
| `apps/macos/src/host/config/mod.rs` | 加 3 行 | `tick()` 里刷新子菜单 |
| `apps/macos/src/host/settings.rs` | 加几行 | 子菜单动作转给 `qingjian_cloud_mac::perform`，之后重套配置（重新加载了 Cloud 配置的话云联想要换端点）；`[predict] provider` 的保存 |
| `apps/macos/src/host/config/predict.rs` | 新文件 | 云联想生效的配置：`provider = qingjian` 时地址与令牌来自 `qingjian_cloud_mac::llm_endpoint()` |
| `apps/macos/src/preferences/pages/cloud.rs` | 改 | 「服务」弹出菜单：素笺云 / 自定义接口，后者才显示地址、模型、密钥 |
| `apps/macos/src/host/config/mod.rs` | 加 4 行 | `tick()` 里问 `qingjian_cloud_mac::take_input_log_reset()`，换账号清了输入日志后丢掉写入端缓冲里旧账号的输入并重开 |
| `apps/macos/src/host/diagnostics.rs` | 重构 + 加 7 行 | 「清空输入日志」拆出 `truncate_and_reopen_input_log`，新增 `reset_input_log_after_account_switch` 复用它 |
| `apps/macos/src/host/config/predict.rs`、`apps/macos/src/preferences/pages/cloud.rs` | 各改 1 段文案 | 提示改成「登录并打开「大模型（云联想）」」，不再说填服务器地址与设备令牌（只改字符串） |
| `apps/macos/src/app/logging/mod.rs` | 改 1 行 | debug 级别下把 ureq 压到 info |
| `apps/macos/pkg/scripts/postinstall` | 加 8 行 | 清掉旧版单独装的 `QingjianCloud.app` 与登录项 |

**IMK 的坑**：带子菜单的父项排在隐藏项（「有新版本」）之后，IMK 整理菜单时就 `CFRelease(NULL)` 崩，输入法打不了字（271 / 273）。
`cloud/scripts/imk-menu-repro.swift` 不装输入法就能复现，改菜单结构前先跑它：`ok` / `separators` / `dynamic` 要 exit 0，`after-hidden` exit 133。

Windows、Linux 的 Server 不受影响（`merge_remote` 有缺省实现）；以后要接入时各加一个同样的 `inbox` 挂钩。

### 品牌改名与图标（2026-10-04，只改资源与用户可见字符串，标识符、路径、bundle id 一律不动）

| 文件 | 改动 | 说明 |
|---|---|---|
| `apps/macos/Info.plist` | 改 6 行 | 显示名、英文名 `Sujian`、四处菜单图标键改指 `qingjian-menu.tiff` |
| `apps/macos/resources/{en,zh-Hans}.lproj/InfoPlist.strings` | 改 3 行 | 输入源名：素笺 / Sujian |
| `apps/macos/scripts/bundle.sh` | 改 4 行 | icns 源换成 `cloud/brand/icon/macos-1024.png`，菜单图标拷 `macos-menu.tiff` |
| `apps/macos/scripts/uninstall.sh` | 改 2 行 | 打印给用户的话 |
| `apps/macos/README.md` | 改 1 行 | 装机说明 |
| `apps/macos/pkg/distribution.xml` | 改 1 行 | 安装器标题 |
| `apps/macos/pkg/resources/{welcome,conclusion}.html` | 各改 1～2 行 | 欢迎页与结束页 |
| `apps/macos/pkg/scripts/postinstall` | 改 3 行 | 输出文字 |
| `apps/macos/src/preferences/pages/about.rs` | 改文案 | 品牌词；许可说明开头加「基于开源的青简输入法（GPL-3.0）」与源码链接；`REPOSITORY_URL` 指向 `github.com/dayuer/qingjian` |
| `apps/macos/src/{main.rs,menubar/menu.rs,host/diagnostics.rs}`、`preferences/{window.rs,file_dialog.rs,pages/{candidates,dictionaries,usage}.rs}` | 各改字符串 | 菜单「素笺 版本」、偏好设置标题、各页提示、诊断信息首行 |
| `apps/macos/tests/info_plist.rs` | 改 2 处断言文案 | 图标说明与失败信息 |
| `apps/macos/Info.plist` | 再加 2 行 | `LSHasLocalizedDisplayName = true`，访达按本地化名显示「素笺」 |
| `apps/macos/Info.plist`、`scripts/{bundle,uninstall}.sh`、`pkg/scripts/postinstall` | 包名改 Sujian.app | `CFBundleDisplayName` 与文件名主干一致为 `Sujian`（访达才换成本地化名）；安装与卸载顺手清改名前的 `Qingjian.app`（同 bundle id）；`--install` 后注销构建目录那份 |
| `apps/macos/Info.plist`、`scripts/bundle.sh` | 菜单图标改回 PDF | 四个图标键指向 `qingjian-menu.pdf`，源自 `cloud/brand/icon/macos-menu.pdf`（线框白纸，22×16pt）；位图 tiff 时 `TISIconIsTemplate` 不生效，深色菜单栏上仍是黑的 |
| `apps/macos/scripts/bundle.sh` | 构建目录改 `target/macos.noindex/` | Spotlight 不进 .noindex，LaunchServices 不登记构建出的同 id 包，免得系统切换时拉起它、出现两个进程；`--install --register` 用已安装那份注册，最后查只有它在跑 |
| `apps/macos/scripts/bundle.sh`、`tools/release/releases_json.py`、`docs/user/getting-started/install.md` | 安装包名改 sujian- | 成品 `sujian-<版本>-macos-<arch>.pkg`；版本列表两种前缀都认 |
| `apps/macos/src/preferences/{pages/about.rs,pages/mod.rs,mod.rs,setting/mod.rs}`、`host/settings.rs` | 删「官网」按钮 | 去掉 `WEBSITE_URL`、`Setting::OpenWebsite` 与分发（上游官网会误导）；素笺有官网后再加回 |
| `apps/macos/pkg/resources/{welcome,conclusion}.html`、`README.md` | 改 | 安装器页面不写路径，改「卸载方法见 README」；README 卸载一节补路径清单 |

### 上下文预测（素笺，设计见 cloud/docs/specs/2026-10-04-context-prediction-design.md）

评测集 `cloud/data/eval/context-pairs.tsv`（同拼音不同上文），命令 `qingjian-cli --eval-context`；计划与评测记录在 `cloud/docs/plans/2026-10-04-context-prediction.md`。

| 文件 | 改动 | 说明 |
|---|---|---|
| `apps/cli/src/latency.rs` | 新文件 | 耗时分位数 |
| `apps/cli/src/eval/context.rs` | 新文件 | `--eval-context` 的实现与报告 |
| `apps/cli/src/eval/mod.rs` | 加 1 行 | `pub mod context;` |
| `apps/cli/src/args.rs` | 加 2 个参数 | `--eval-context`、`--eval-context-details` |
| `apps/cli/src/main.rs` | 加 1 行 mod、1 个分支 | 分派 |
| `apps/cli/src/replay/mod.rs`、`report.rs` | 加约 12 行、1 个字段 | 回放报按键同步 p50 / p99；`--neural-async` 时像 `--eval-text` 一样等重排再查一次 |
| `crates/qingjian-core/src/engine/query/left_context.rs` | 新文件 | 前文末尾 8 字 → `Context`；`Engine::word_context`（链优先，私密不看） |
| `crates/qingjian-core/src/engine/query/mod.rs` | 加 1 行 | `mod left_context;` |
| `crates/qingjian-core/src/engine/query/phonetic.rs` | 改 2 行 | 词级排序的上下文改用 `word_context()`；整句首词不看前文（变体对比见计划文件） |
| `crates/qingjian-core/src/engine/tests/context_fork.rs`、`tests/mod.rs` | 新文件、加 1 行 | 测试 |
| `apps/macos/src/imk/controller/display.rs` | 改 4 行 | 第一键总是读应用前文（私密不读），不再只在有模型时读 |
| `apps/macos/src/host/model/mod.rs` | 删 1 个方法 | `model_loading` 不再有人用 |
| `crates/qingjian-core/src/ranking/choice_bonus.rs` | 新文件 | β=8 与 `choice_bonus` |
| `crates/qingjian-core/src/ranking/mod.rs` | 加 `rank`（β 加分），原 `rank` 改名 `rank_choice_first` | 拼音词级走加分，形码保留「选过的次数排在上下文得分前面」的旧键 |
| `crates/qingjian-core/src/engine/query/phonetic.rs` | 再改 1 行 | 调 `ranking::rank(…, self.choice_bonus, …)` |
| `crates/qingjian-core/src/engine/query/code.rs` | 改 1 行 | 调 `ranking::rank_choice_first`（行为不变，上游形码测试断言没动） |
| `crates/qingjian-core/src/engine/mod.rs`、`setup.rs` | 加 1 个字段、2 个方法 | `choice_bonus`、`set_choice_bonus`（回放调参） |
| `apps/cli/src/tuning.rs`、`args.rs` | 加 1 个键、1 行文档 | `--tune choice=β` |
| `cloud/scripts/choice-sweep.sh` | 新文件 | 扫 β 的脚本 |
| `crates/qingjian-core/src/engine/rescoring/word_rescore/mod.rs`、`tests.rs` | 新文件 | 知微词级重排：取档、请求、收结果；测试 |
| `crates/qingjian-core/src/engine/rescoring/mod.rs` | 加 1 行 mod、1 行导出，改 3 个入口，加 `rescoring_in_flight` | `rescoring_pending` / `request_rescoring` / `poll_rescoring` 带上知微；记「在飞」任务序号 |
| `crates/qingjian-core/src/engine/rescoring/worker.rs` | 加 `id` 字段与 `AtomicU64`，`submit` 返回序号 | 任务序号，供 `rescoring_in_flight` 判断最新一条回来没有 |
| `crates/qingjian-core/src/engine/mod.rs` | 加 6 个字段 | `word_scorer` / `word_rescorer` / `word_cache` / `word_weight` / `sentence_awaiting` / `word_awaiting` |
| `crates/qingjian-core/src/engine/setup.rs` | 加 1 行 | 换 / 卸整句重打分器时清 `sentence_awaiting` |
| `crates/qingjian-core/src/ranking/mod.rs` | 改返回值 | `rank` / `rank_choice_first` 返回每条的最终分，给词级重排当静态分 |
| `crates/qingjian-core/src/engine/query/phonetic.rs` | 再加 3 行 | 取档（`word_tier`）、调 `rescore_first_page` |
| `apps/cli/src/rescoring.rs` | 改 `settle` | 等 `rescoring_in_flight` 归零而不是第一次收到结果 |
| `apps/cli/src/args.rs`、`main.rs` | 加 2 个参数与加载 | `--word-model`、`--word-weight` |
| `apps/macos/src/host/model/mod.rs` | 改 `poll_rescoring` 1 处 | 另一条线程还在算就继续轮询 |
| `cloud/scripts/word-sweep.sh` | 新文件 | 扫 λ_w 的脚本 |
| `crates/qingjian-platform/src/config/scorer_set.rs` | 新文件 | `ScorerSet`（`tongbian` / `both`） |
| `crates/qingjian-platform/src/config/model.rs`、`mod.rs`、`lib.rs` | 加 1 个字段、2 行模板、1 条测试、导出 1 个名字 | `[model] scorers`，缺省 `both` |
| `apps/macos/src/host/model/word_model.rs` | 新文件 | 知微后台加载、接上（晚于组句时补查）、卸掉 |
| `apps/macos/src/host/model/mod.rs` | 再加 1 行 mod、改 4 处 | `load_local_model` 先管知微；`attach_loaded_model` 兼看知微还在加载；`unload_local_model` 卸知微 |
| `apps/macos/src/host/mod.rs`、`init.rs` | 加 1 个字段 | `word_loader` |
| `cloud/docs/design.md` | 加一节 | 上下文预测 |
| `crates/qingjian-neural/src/continuation.rs` | 新文件 | `CharScorer::continue_text` 贪心续写 |
| `crates/qingjian-neural/src/lib.rs` | 加 1 行 | `mod continuation;` |
| `crates/qingjian-neural/src/core_scorer.rs` | 加 1 个方法 | `SentenceScorer::continue_text` 的实现 |
| `crates/qingjian-core/src/sentence/scorer.rs` | 加 1 个缺省方法 | `continue_text` |
| `apps/cli/src/eval/continuation.rs` | 新文件 | `--eval-continuation` |
| `apps/cli/src/eval/mod.rs`、`args.rs`、`main.rs` | 各加几行 | 挂进去 |

### 按人隔离也包括新造词（素笺，Core 只加缺省为空的挂钩）

审计 2026-10-05：`ScopedLearner` 的 `learn_word` / `learn_english` 一律写全局，在某个人那里造的词在别人那里也出。
对象层锁在 `Mutex` 里，`Learner::user_words(&self)` 只能返回一份引用，所以 Core 加三个缺省为空的方法，桥里存对象层用户词的快照（`scope/snapshot.rs`）。
上游已有的 `Learner` 实现（`FrequencyLearner`、`MutedLearner`、`NoLearner`）一行不改：Engine 经 `MutedLearner::inner()` 直接问里面那个学习器，
`scope_changed` 经 `Engine::learner_mut()`（本来就给的是里面那个）调。

| 文件 | 改动 | 说明 |
|---|---|---|
| `crates/qingjian-core/src/engine/learning/learner.rs` | 加 3 个缺省方法 | `scoped_user_words` / `scoped_user_english` 缺省 `None`，`scope_changed` 缺省什么都不做 |
| `crates/qingjian-core/src/engine/setup.rs` | 加 4 行 | `all_dictionaries` 多推一份 `inner().scoped_user_words()` |
| `crates/qingjian-core/src/engine/extras.rs` | 改 3 行 | `english_lists` 把 `inner().scoped_user_english()` 放最前 |
| `crates/qingjian-core/src/engine/query/english_tail.rs` | 加 2 行 | 句尾英文的「个人表里有」也看叠加层那份 |

缺省实现下没有叠加层，`--eval-text`（1878 句）与 `--replay`（2026-10-04 日志）改前改后逐字相同，逐键计时 p99 不变（数字在提交说明里）。

### 整句词图格子放宽 + 词库补数字复合词（Core + 数据，各平台都生效）

2026-10-06 用户报告：`jishimutiandi`（几十亩田地）首选「即使木天地」，`jibaimu`（几百亩）首选「击败木」——而 几百 本来就在词库里。
根因两层：`sentence::SPAN_CANDIDATES = 6` 按词库词频截格子，低频但语境正确的字（亩，mu 单字格里词频排不进前 6）根本进不了词图；
「几十」则在常用词表源、领域补充、短语层、当时语料挖词的产物里全部缺失。

| 文件 | 改动 | 说明 |
|---|---|---|
| `crates/qingjian-core/src/sentence/mod.rs` | 改 1 行 | `SPAN_CANDIDATES` 6→12（注释里写了为什么） |
| `assets/lexicon/dict.tsv` | 加 4 行 | 几十 / 第六 / 几亿 + 日期注释，词频按 jieba 折算量级人工定 |
| `assets/lexicon/domain_words.tsv` | 加 4 行 | 同步补进源，下次全量重跑 lexicon 不丢 |

修复后三例首选翻正（几十亩田地 / 几百亩 / 第六），查询延迟均值不变（4.6ms）。
代价如实记：整句评测（1878 句冷启动）首选掉约 1.6 个点——放宽引进的词静态模型分不清，神经重打分（top-6 路径）也只能裁决一部分：

| 配置 | 首选 | 前三 | 字准 |
|---|---|---|---|
| SPAN=6 无神经 | 32.7% | 38.3% | 76.8% |
| SPAN=12 无神经 | 31.3% | 36.3% | 75.9% |
| SPAN=6 + 通变 P2C | 37.5% | 40.1% | 79.1% |
| SPAN=12 + 通变 P2C | 35.9% | 38.4% | 78.3% |

权衡取 12：要的是「推出来至少得是正常的词」这条底线——基准掉点集中在放宽后引进的似是而非句，而埋词类失败（击败木）是质变。回滚或调优只动这一个常数。

> **2026-10-06 追记：已回退到 6。** 同日合并的 admission（PR #7，`SPAN_POOL` 候补池按前文抬举）用更准的方式解决同一问题；
> SPAN=12 会让罕见读音（和/huo）无条件进图、绕过 admission 的语义（`context_does_not_admit_a_rare_reading_of_a_polyphone` 挂）。
> 回退后三例照旧首选第一（几十亩田地 / 几百亩 / 第六，补词与神经不动），评测回到 SPAN=6 基线，上表的掉点一并消除。
> 上表的文件清单里 `sentence/mod.rs` 那行随之作废，词库补词两行保留。

## 合并上游时

1. 冲突只可能出在上表「加 N 行」的那几个文件，按上游的新写法把挂钩行重新加回去。
2. 上游改了 `FrequencyLearner` 的字段或 `UserNgram` 的文本格式：跑 `cargo test -p qingjian-learning remote`
   与 `cd cloud && cargo test --test learning`（后者用真正的学习模块端到端验证），红了就改 `remote.rs`。
3. 上游新加了学习表：`remote.rs` 与 `cloud/crates/qingjian-cloud-client/src/learning/table.rs` 各加一种。
