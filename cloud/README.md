# 青简 Cloud

素笺（青简输入法的云端扩展）的客户端：用 Apple ID 或邮箱登录后，跨设备剪贴板、学习数据与设置同步、输入历史汇总，以及用大模型补候选、做润色，各项单独开启、缺省全关。登录前要先同意把数据发到境外服务器。支持 macOS 与 iOS。

这个目录和上游青简隔离，有独立的 Cargo workspace。唯一的例外是学习数据同步需要的一小份分叉补丁（对上游已有文件共加 12 行），清单与合并上游时的处理见 [docs/fork-patch.md](docs/fork-patch.md)。
离线时输入法照样可用，Cloud 只锦上添花。设计、隔离规则与分期见 [docs/design.md](docs/design.md)。

上游还没合并的 PR 想先用：编辑 [patches.txt](patches.txt)，运行 `cloud/scripts/build-local.sh` 生成 `local` 分支（main + 这些 PR）。

## 现状

跨设备剪贴板、学习数据与设置同步、大模型代理、输入日志汇总、纠错闭环都已上线；这个目录里是客户端一侧：

| 目录 | 内容 |
|---|---|
| [crates/qingjian-cloud-mac/](crates/qingjian-cloud-mac/) | Mac 端：链进输入法进程的同步模块；[使用与验收清单](crates/qingjian-cloud-mac/README.md) |
| [ios/](ios/) | iOS 主 App + 键盘扩展：本地引擎、26 键全拼，键位照 iOS 自带简体拼音；[构建与已知问题](ios/README.md) |
| [crates/](crates/) | 协议类型与客户端（离线队列、SSE、重连），各平台共用；`qingjian-cloud-bridge` 把上游 Engine 包成 C ABI 给 iOS 键盘；本地整句模型（含章·通变）经 `qj_load_model` 异步加载、接 Engine 的重打分，键盘余量低于 8MB 自动卸载（`qj_available_memory_mb`） |

账号与多租户的设计见 [docs/design.md](docs/design.md) 的「鉴权」与「已知限制」；Mac 对 `apps/macos` 的分叉改动清单见 [docs/fork-patch.md](docs/fork-patch.md)。

服务端、纠错闭环（tuner）与部署是闭源的，2026-10-04 起在独立仓库 `synon-ime`（与本检出并排放），按路径引用这里的协议与客户端 crate。

开发：在 `cloud/` 下 `cargo test`、`cargo clippy --all-targets -- -D warnings`；Mac 程序在 Linux 上可用
`cargo check -p qingjian-cloud-mac --target aarch64-apple-darwin` 做编译检查。改了协议或客户端 crate，也要在 `synon-ime` 里跑一遍测试。
