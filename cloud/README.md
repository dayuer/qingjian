# 青简 Cloud

青简输入法的个人扩展：自建服务器、跨设备剪贴板、输入历史汇总，以及用大模型补候选、修正词库、做联想。支持 macOS 与 iOS。

这个目录和上游青简隔离，有独立的 Cargo workspace。唯一的例外是学习数据同步需要的一小份分叉补丁（对上游已有文件共加 12 行），清单与合并上游时的处理见 [docs/fork-patch.md](docs/fork-patch.md)。
离线时输入法照样可用，Cloud 只锦上添花。设计、隔离规则与分期见 [docs/design.md](docs/design.md)。

上游还没合并的 PR 想先用：编辑 [patches.txt](patches.txt)，运行 `cloud/scripts/build-local.sh` 生成 `local` 分支（main + 这些 PR）。

## 现状

跨设备剪贴板、学习数据与设置同步、大模型代理、输入日志汇总、纠错闭环的代码已完成，待 Mac 真机验收：

| 目录 | 内容 |
|---|---|
| [server/](server/) | 服务端（Rust：axum + SQLite），Docker 镜像约 56 MB |
| [deploy/](deploy/) | docker-compose + Caddy 自动 HTTPS、备份脚本；[部署说明](deploy/README.md) |
| [mac-agent/](mac-agent/) | Mac 菜单栏常驻程序；[安装与验收清单](mac-agent/README.md) |
| [tuner/](tuner/) | 纠错闭环：词库体检、一次就学会、话题补词，回放把关（可选服务） |
| [crates/](crates/) | 协议类型与客户端（离线队列、SSE、重连），各平台共用 |

开发：在 `cloud/` 下 `cargo test`、`cargo clippy --all-targets -- -D warnings`；Mac 程序在 Linux 上可用
`cargo check -p qingjian-cloud-mac --target aarch64-apple-darwin` 做编译检查。tuner 的端到端测试要上游 CLI 与产品数据：
`tools/release/data-fetch.sh && cargo build --release -p qingjian-cli`（仓库根目录），再
`QINGJIAN_CLI=…/target/release/qingjian-cli QINGJIAN_DATA=…/data/generated cargo test -p qingjian-cloud-tuner`；没给就跳过。
