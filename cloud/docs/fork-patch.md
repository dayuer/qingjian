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

### 自建更新服务器（只在 macOS 生效）

输入法查 `https://pingyin.synon.ai/releases/releases.json`，只认自己的签名密钥；有新版就在后台下好 pkg 并校验 sha256，
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

### 青简 Cloud 链进输入法进程（只在 macOS 生效）

青简 Cloud 的 Mac 端是库 `cloud/crates/qingjian-cloud-mac`，输入法按路径依赖它，不另起常驻程序。它自带 0.5 秒的
NSTimer（输入法自己的定时器失焦就停）与后台线程，入口都包 `catch_unwind`，出错只停同步；每 2 秒看当前输入法，
连续 30 秒不是青简就停掉同步线程，切回来恢复。输入法只调三个函数：`start(mtm)`、`menu_lines()` / `menu_revision()`、`perform(tag)`。

| 文件 | 改动 | 说明 |
|---|---|---|
| `Cargo.toml`（根） | 加 2 行 | `exclude = ["cloud"]`：不然按路径引用 cloud 的 crate 时，它的 `*.workspace = true` 会去找根 workspace |
| `Cargo.lock` | 新增条目 | ureq、native-tls 证书等；`uuid` 1.26.1 → 1.27.0（cloud 要求） |
| `apps/macos/Cargo.toml` | 加 2 行 | 依赖 `qingjian-cloud-mac` |
| `apps/macos/src/main.rs` | 加 2 行 | IMKServer 建好后 `qingjian_cloud_mac::start(mtm)` |
| `apps/macos/src/menubar/cloud_agent.rs` | 新文件 | 照菜单行画「青简 Cloud ›」子菜单 |
| `apps/macos/src/menubar/action.rs` | 加 1 个变体 | `MenuAction::CloudAgent(tag)`，tag 1000..1200 |
| `apps/macos/src/menubar/menu.rs` | 加字段与 `sync_cloud_agent` | 父项紧挨「模糊音」（见下面 IMK 的坑） |
| `apps/macos/src/menubar/mod.rs` | 加 1 行 | `mod cloud_agent` |
| `apps/macos/src/host/config/mod.rs` | 加 3 行 | `tick()` 里刷新子菜单 |
| `apps/macos/src/host/settings.rs` | 加 1 行 | 子菜单动作转给 `qingjian_cloud_mac::perform` |
| `apps/macos/src/app/logging/mod.rs` | 改 1 行 | debug 级别下把 ureq 压到 info |
| `apps/macos/pkg/scripts/postinstall` | 加 8 行 | 清掉旧版单独装的 `QingjianCloud.app` 与登录项 |

**IMK 的坑**：带子菜单的父项排在隐藏项（「有新版本」）之后，IMK 整理菜单时就 `CFRelease(NULL)` 崩，输入法打不了字（271 / 273）。
`cloud/scripts/imk-menu-repro.swift` 不装输入法就能复现，改菜单结构前先跑它：`ok` / `separators` / `dynamic` 要 exit 0，`after-hidden` exit 133。

Windows、Linux 的 Server 不受影响（`merge_remote` 有缺省实现）；以后要接入时各加一个同样的 `inbox` 挂钩。

## 合并上游时

1. 冲突只可能出在上表「加 N 行」的那几个文件，按上游的新写法把挂钩行重新加回去。
2. 上游改了 `FrequencyLearner` 的字段或 `UserNgram` 的文本格式：跑 `cargo test -p qingjian-learning remote`
   与 `cd cloud && cargo test --test learning`（后者用真正的学习模块端到端验证），红了就改 `remote.rs`。
3. 上游新加了学习表：`remote.rs` 与 `cloud/crates/qingjian-cloud-client/src/learning/table.rs` 各加一种。
