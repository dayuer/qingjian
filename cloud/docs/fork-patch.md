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

## 合并上游时

1. 冲突只可能出在上表「加 N 行」的那几个文件，按上游的新写法把挂钩行重新加回去。
2. 上游改了 `FrequencyLearner` 的字段或 `UserNgram` 的文本格式：跑 `cargo test -p qingjian-learning remote`
   与 `cd cloud && cargo test --test learning`（后者用真正的学习模块端到端验证），红了就改 `remote.rs`。
3. 上游新加了学习表：`remote.rs` 与 `cloud/crates/qingjian-cloud-client/src/learning/table.rs` 各加一种。
