# 青简 Cloud

青简输入法的个人扩展：自建服务器、跨设备剪贴板、输入历史汇总，以及用大模型补候选、修正词库、做联想。支持 macOS 与 iOS。

这个目录和上游青简完全隔离：不修改目录以外的任何文件，有独立的 Cargo workspace。合并上游时，本目录以外的部分直接取上游版本。
离线时输入法照样可用，Cloud 只锦上添花。设计、隔离规则与分期见 [docs/design.md](docs/design.md)。

上游还没合并的 PR 想先用：编辑 [patches.txt](patches.txt)，运行 `cloud/scripts/build-local.sh` 生成 `local` 分支（main + 这些 PR）。

还在设计阶段，尚无可用代码。
