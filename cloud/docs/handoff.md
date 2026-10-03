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
| 1 | 跨设备剪贴板：服务端（axum + SQLite、设备令牌、SSE）、Docker + Caddy、Mac 菜单栏常驻程序 `mac-agent` | 端到端测试；经 Caddy 推送 23 ms | **未做**，清单在 `mac-agent/README.md` |
| 1.5 | 学习数据（词频、选择、n-gram、敲错、英文词、用户词）与 `config.toml` 同步：基线 + 增量 + 收件箱；分叉补丁让输入法合并收件箱 | 用上游真正的 `FrequencyLearner` 模拟两台设备的端到端测试 | **未做**；分叉补丁里的 Mac 壳改动在 Linux 上编译不了 |
| 2 | 大模型代理（换密钥、缓存、用量、可选跨设备上文，缺省关）、输入日志上传 / 下载 / 清空传播；`mac-agent`「让青简使用 Cloud 的大模型」 | 假上游测试；日志续传与清空测试 | **未做**，没接过真实 DeepSeek |
| 3 | 纠错闭环 `tuner`：词库体检、一次就学会、话题补词，用上游 `qingjian-cli --replay` 做门槛；可选的 compose profile，缺省只出报告 | 真服务器 + 真 CLI 与 data-v3 + 假上游的端到端测试（留出段 50% → 100%）；镜像在沙箱里构建并在 compose 里跑通 | **未做** |

部署：空机器用 `cloud/deploy/install.sh`（Cloudflare 改 DNS、装 Docker、拉代码、compose + Caddy 启动、登记设备、可选 `ENABLE_TUNER=1`）；80/443 已被 nginx 占用的机器用 `cloud/deploy/deploy-nginx.sh`。

## 线上部署

- VPS 43.156.128.95，域名 `pingyin.synon.ai`（用户给的拼写，保留），2026-10-03 上线，跑的是 `ab9c16a`。
- 用的是 `cloud/deploy/deploy-nginx.sh`，不是 `install.sh`：那台机器的 80/443 由宿主机 nginx 占用，证书由 certbot 管。
  服务器上：代码在 `/opt/qingjian`；本机专用的 compose 覆盖文件、Dockerfile 与备份脚本在 `/opt/qingjian-host/`；
  cloud 绑定 `127.0.0.1:18100`；nginx 站点在 `/etc/nginx/sites-available/pingyin.synon.ai`；
  `.env` 里的 `COMPOSE_FILE` 指向覆盖文件，所以在 `cloud/deploy` 下直接执行 `docker compose …` 即可；每天 04:30 备份到 `/root/qingjian-backups`。
- 已登记的设备：`macbook`，令牌在这台 Mac 的 `~/Library/Application Support/QingjianCloud/config.toml`。
- 升级：在本机重新运行 `cloud/deploy/deploy-nginx.sh`（不带 `CF_API_TOKEN` 就不改 DNS）。
  第 3 期纠错闭环（`6051d3e`）还没部署，`deploy-nginx.sh` 也还没接 `ENABLE_TUNER`。
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
