# 青简 Cloud

青简输入法的个人扩展：自建服务器、跨设备剪贴板、输入历史汇总，以及用大模型补候选、修正词库、做联想。支持 macOS 与 iOS。

这个目录和上游青简完全隔离：不修改目录以外的任何文件，有独立的 Cargo workspace。合并上游时，本目录以外的部分直接取上游版本。
离线时输入法照样可用，Cloud 只锦上添花。设计、隔离规则与分期见 [docs/design.md](docs/design.md)。

上游还没合并的 PR 想先用：编辑 [patches.txt](patches.txt)，运行 `cloud/scripts/build-local.sh` 生成 `local` 分支（main + 这些 PR）。

## 现状

第 1 期（跨设备剪贴板）代码已完成，待 Mac 真机验收：

| 目录 | 内容 |
|---|---|
| [server/](server/) | 服务端（Rust：axum + SQLite），Docker 镜像约 46 MB |
| [deploy/](deploy/) | docker-compose + Caddy 自动 HTTPS、备份脚本；[部署说明](deploy/README.md) |
| [mac-agent/](mac-agent/) | Mac 菜单栏常驻程序；[安装与验收清单](mac-agent/README.md) |
| [crates/](crates/) | 协议类型与客户端（离线队列、SSE、重连），各平台共用 |

开发：在 `cloud/` 下 `cargo test`、`cargo clippy --all-targets -- -D warnings`；Mac 程序在 Linux 上可用
`cargo check -p qingjian-cloud-mac --target aarch64-apple-darwin` 做编译检查。
