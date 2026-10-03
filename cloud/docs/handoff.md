# 交接（2026-10-03）

青简 Cloud 第一个开发会话的收尾：做了什么、验证到哪一步、下一步从哪接手。设计见 [design.md](design.md)，分叉补丁见 [fork-patch.md](fork-patch.md)。

## 分支

| 分支 | 内容 |
|---|---|
| `claude/gallant-brown-c1v3zi` | 全部开发：`cloud/` 目录 + 分叉补丁（对上游已有文件共加 12 行）。基于上游 main `c08ae57`，没有分叉以外的改动 |
| `local` | 自用：上面的分支 + 上游 5 个 PR（#290 #324 #244 #256 #348），由 `cloud/scripts/build-local.sh` 生成，不要在上面直接提交 |

重建 `local`：`BASE=origin/claude/gallant-brown-c1v3zi PUSH=1 cloud/scripts/build-local.sh`。PR 清单在 `cloud/patches.txt`。

## 已完成

| 期 | 内容 | 自动化验证 | 真机 / 线上验证 |
|---|---|---|---|
| 1 | 跨设备剪贴板：服务端（axum + SQLite、设备令牌、SSE）、Docker + Caddy、Mac 菜单栏常驻程序 `mac-agent` | 端到端测试；经 Caddy 推送 23 ms | 单台 Mac + 模拟设备：上传通；别的设备推的 0.52 s 进本机剪贴板 |
| 1.5 | 学习数据（词频、选择、n-gram、敲错、英文词、用户词）与 `config.toml` 同步：基线 + 增量 + 收件箱；分叉补丁让输入法合并收件箱 | 用上游真正的 `FrequencyLearner` 模拟两台设备的端到端测试 | 单台 Mac + 模拟设备：真实打字的词频 / 选择 / n-gram / 敲错表上传通；别的设备的增量经收件箱并进输入法，不回推、不翻倍；`config.toml` 上传通 |
| 2 | 大模型代理（换密钥、缓存、用量、可选跨设备上文，缺省关）、输入日志上传 / 下载 / 清空传播；`mac-agent`「让青简使用 Cloud 的大模型」 | 假上游测试；日志续传与清空测试 | 真实 DeepSeek 经代理通；输入法云联想走 Cloud 出整句补全；输入日志上传通 |
| 3 | 纠错闭环 `tuner`：词库体检、一次就学会、话题补词，用上游 `qingjian-cli --replay` 做门槛；可选的 compose profile，缺省只出报告 | 真服务器 + 真 CLI 与 data-v3 + 假上游的端到端测试（留出段 50% → 100%）；镜像在沙箱里构建并在 compose 里跑通 | **未做** |

部署：空机器用 `cloud/deploy/install.sh`（Cloudflare 改 DNS、装 Docker、拉代码、compose + Caddy 启动、登记设备、可选 `ENABLE_TUNER=1`）；80/443 已被 nginx 占用的机器用 `cloud/deploy/deploy-nginx.sh`。

## 真机验收（2026-10-03，单台 MacBook）

验收中发现并修掉的：

- `e4cf940`：mac-agent 一发 https 请求就 panic（ureq 选 NativeTls 要开 `native-tls` 特性，原先只开了 `-no-default`）。
  同步线程 panic 后静默退出，菜单停在「连接中」，日志里没有任何报错。自动化测试走 Linux + rustls，覆盖不到这条路径。
- `9946a68`：`backup.sh` 在容器里执行 `rm`，distroless 镜像没有这个命令。

装机时踩到的（不是 bug，但要写进安装说明）：

- `apps/macos/scripts/bundle.sh --install` 装上的输入法，`--register` 之后上级输入源仍未启用，系统设置里也找不到；**注销后重新登录**才出现。
- Claude 会话的沙箱里 `pbcopy` 写不进系统剪贴板，测剪贴板要在沙箱外用 `osascript -e 'set the clipboard to …'`。

还没验的：两台真实 Mac 之间互传（目前用临时设备 `test-b` 拿 curl 模拟）、断网后补传、合盖后恢复、密码不上传、
输入日志清空的传播、`qingjian-cloud usage` 里的用量统计、`shiguo` 选词后排序在另一台机器上生效。

后续可做：同步线程 panic 时记一条日志并在菜单上显示错误，不要静默退出。

## 自用补丁与自建更新（2026-10-03）

- 「回车上屏高亮候选」开关（`0e15e56`）：`[general] enter_commits_candidate`，偏好设置「通用」页勾选，只在 macOS 生效。
- 自动更新改走 `https://pinyin.synon.ai/releases/`（`61a759a`）：只认自己的签名密钥，查到新版后在后台下好 pkg 并校验 sha256，
  点菜单「有新版本」打开安装程序；pkg 里带着青简 Cloud，postinstall 一起装上。清单见 [fork-patch.md](fork-patch.md)。
- 发布：在 `local` 分支的干净检出里运行 `NOTES="…|…" cloud/scripts/publish-mac.sh`，版本号是 `<上游版本>-local.<提交数>`。
  签名私钥 `~/.config/qingjian-cloud/release-signing.key` 只在这台 Mac 上，**丢了就再也发不了更新**。
- 已在真机验过：`0.1.5-local.268` 已发布并用 pkg 装上（输入法在 `/Library/Input Methods`，青简 Cloud 跟着装上并重启），
  启动后到自建服务器检查更新，结果是「已是最新」；在本机模拟旧版本查到了新版，139 MB 的 pkg 下载完整、校验通过。
  `0.1.5-local.271` 用自动更新装上：「立即检查」查到、后台下好并校验、点「有新版本」打开安装程序、装完青简 Cloud 跟着更新重启。
- 青简 Cloud 并进「中☁ → 青简 Cloud ›」子菜单、只在用青简时运行（`3519a20`），本机验过退出与拉起。
- 271 / 273 一打开输入源菜单就崩（IMK 内部 `CFRelease(NULL)`）：带子菜单的父项不能排在隐藏项之后，274 修好。
  规则用 `cloud/scripts/imk-menu-repro.swift` 验过，改菜单结构前先跑它。
- 输入法连崩几次之后 macOS 会不再拉它的进程、悄悄切回别的输入法，没有任何日志；重启 `imklaunchagent` 也救不回来，
  **注销重新登录**才恢复。另外构建目录里的 `target/Qingjian.app` 等同 id 副本会被 LaunchServices 登记上，`publish-mac.sh` 现在打完包就注销它们。
- 回车上屏开关（`0e15e56`）还没听到用户反馈。
- **青简 Cloud 链进输入法进程**（用户不要多一个常驻程序）：`cloud/mac-agent` 改成库 `cloud/crates/qingjian-cloud-mac`，
  输入法启动时 `start`，「中☁ → 青简 Cloud ›」照 `menu_lines()` 画；不再有 `QingjianCloud.app`、LaunchAgent、`menu.txt`、`commands/`，
  pkg 的 postinstall 清掉旧的。剪贴板同步按用户要求保留（与苹果通用剪贴板的防回灌在 `service.rs`）。表里早先写的 `mac-agent` 就是它的前身。
- 踩过的坑：脚本里写 `"$HOST）"`，部分 locale 下 bash 会把全角括号的首字节算进变量名（`73e6e88`）；
  nginx 的 `alias` 里不要嵌正则 location，json 会 404（`8be4759`）。

## 线上部署

- VPS 43.156.128.95，域名 `pinyin.synon.ai`（2026-10-03 从拼错的 `pingyin.synon.ai` 改过来，旧域名已撤，见下），2026-10-03 上线，跑的是 `ab9c16a`。
- 用的是 `cloud/deploy/deploy-nginx.sh`，不是 `install.sh`：那台机器的 80/443 由宿主机 nginx 占用，证书由 certbot 管。
  服务器上：代码在 `/opt/qingjian`；本机专用的 compose 覆盖文件、Dockerfile 与备份脚本在 `/opt/qingjian-host/`；
  cloud 绑定 `127.0.0.1:18100`；nginx 站点在 `/etc/nginx/sites-available/pinyin.synon.ai`；
  `.env` 里的 `COMPOSE_FILE` 指向覆盖文件，所以在 `cloud/deploy` 下直接执行 `docker compose …` 即可；每天 04:30 备份到 `/root/qingjian-backups`。
- 改域名（2026-10-04 完成）：`0.1.5-local.288` 起检查更新走新域名；本机装上 288 后，`releases.json` 里旧版本的下载地址改成新域名并重签，
  证书重签为只含 `pinyin.synon.ai`，旧证书、nginx 的旧 `server_name` 与 Cloudflare 上的 `pingyin` 记录都已删除。286 及之前的版本从此查不到更新。
- 已登记的设备：`macbook`（令牌在这台 Mac 的 `~/Library/Application Support/QingjianCloud/config.toml`）、`iphone`（令牌在 `cloud/ios/cloud.local.toml`，不进仓库）。
- 升级：在本机重新运行 `cloud/deploy/deploy-nginx.sh`（不带 `CF_API_TOKEN` 就不改 DNS）。
- **纠错闭环（tuner）2026-10-04 上线**，每 6 小时一轮、推送已打开（`.env` 的 `QINGJIAN_TUNER_DRY_RUN=false`，改回 true 只出报告）。
  `deploy-nginx.sh` 还没接 `ENABLE_TUNER`，是手动部署的：本机 `git archive HEAD` 传到服务器 `/opt/qingjian-tuner-src`（不经公开的 GitHub 仓库），
  在那里 `docker build -f cloud/tuner/Dockerfile -t qingjian-cloud-tuner:latest .`，再在 `/opt/qingjian/cloud/deploy` 下
  `docker compose --profile tuner up -d --no-build tuner`。登记的设备是 `tuner`，令牌在 `.env`。看报告：`docker compose --profile tuner logs tuner`。
  第一轮（日志 431 行）采纳 3 条话题补词，回放首选命中 93.3% → 93.3%。iPhone 从 `15d2f84` 起也上传输入日志。
- 待办：作废聊天里贴过的 Cloudflare 令牌，换成只能改 `synon.ai` DNS 的令牌。
- `install.sh` 默认 80/443 端口空闲；在已有反向代理的机器上要用 `deploy-nginx.sh`。以后可以给 `install.sh` 加 `PROXY=external` 模式，把两个脚本合成一个。

## 第 4 期（iOS 键盘）：只做了调研，没写代码

确认过的：

- 上游 Core、学习、语言模型、词库、释义 crate 都能 `cargo check --target aarch64-apple-ios`（没有要交叉编译的 C 依赖）；`aarch64-apple-ios`、`aarch64-apple-ios-sim` 已能装。
- 壳驱动引擎的方式参照 `apps/linux/server`：`Engine::new(Dictionary)` + `with_learner(FrequencyLearner)` + `with_language_model(BigramModel::from_path("lm.qj"))`（见 `assembly/`）；
  按键 `push` / `backspace` / `take_raw` / `punctuate`，候选 `query()` → `candidates.items`，上屏 `commit(&candidate)`，落盘 `flush_learning`（见 `dispatch/composed/mod.rs`）。

打算的做法（未实现）：

- `cloud/ios/core`：Rust crate，`staticlib` + `rlib`，C 接口用 JSON 传事件与状态（`key` / `backspace` / `space` / `enter` / `select` / `punct` / `toggle_english` → `{insert, delete_backward, state{composing, preedit, candidates, english}}`）；
  内含收件箱合并（同 Mac 壳的 `host/cloud/inbox.rs`）、输入日志、`ClipboardSync` 与 `DataSync`（客户端 crate 用 `native-tls`）。可在 Linux 上用真实数据写测试。
- `cloud/ios`：XcodeGen `project.yml`；主 App（SwiftUI，填服务器与令牌、写 App Group）；键盘扩展（SwiftUI 26 键全拼 + 横滑候选栏 + 剪贴板候选 + 地球键）；
  数据文件放键盘扩展的资源里 mmap；学习数据、`config.toml`、收件箱放 App Group。
- 要在 Mac 上验证的风险：键盘扩展的内存上限（约 48–60 MB，`lm.qj` 29 MB 走 mmap）、完全访问下读剪贴板的系统提示。

## 本地复现

```bash
cd cloud && cargo test && cargo clippy --all-targets -- -D warnings
cargo clippy -p qingjian-cloud-mac --target aarch64-apple-darwin -- -D warnings
# tuner 端到端测试（仓库根目录先准备 CLI 与数据，没有就自动跳过）
tools/release/data-fetch.sh && cargo build --release -p qingjian-cli
QINGJIAN_CLI=$PWD/target/release/qingjian-cli QINGJIAN_DATA=$PWD/data/generated cargo test --manifest-path cloud/Cargo.toml -p qingjian-cloud-tuner
```

## 用户提过、还没排期的

- 微信输入法截图里的功能：符号自动补全、自动编号、中英 / 中数自动加空格、`;` `'` 选第 2 / 3 个候选、翻页 `-` `=`、Mac 单击 Shift 切中英（上游 #291）、按应用默认英文。都要改输入法，做成小补丁。
- Mac 上屏后联想（要改 Core）。
- 生词本（`user-vocab.tsv`）同步：用户说核心不是学英语，暂不做。
