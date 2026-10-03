# iOS 键盘

青简 Cloud 的 iOS 主 App 与键盘扩展。键盘里跑完整的本地引擎（`qingjian-cloud-bridge` 编成的静态库）。
「完全访问」用于按键震动（iOS 规定第三方键盘没有它不能震）与连青简 Cloud；不开照常离线打字、只有系统键盘音。

键位与交互照 iOS 26 自带的简体拼音 26 键：

- 三层键位：字母、`123`（数字与 `。，、？！` 等中文标点）、`#+=`；
- 拼音以 marked text 写在宿主的光标处；
- 空格上屏首选，换行原样上屏字母（打英文靠它）；
- 候选栏右端 ⌄ 展开全部候选；😀 打开自带的表情面板。

## 构建

```bash
brew install xcodegen
cd cloud/ios
xcodegen generate                 # 生成 QingjianCloud.xcodeproj（不进仓库）
open QingjianCloud.xcodeproj
```

Keyboard target 的 preBuildScript 会跑 `scripts/build-bridge.sh`，它做两件事：

- 编出真机加模拟器的 `Frameworks/QingjianBridge.xcframework`；
- 把 `dict.qj`、`lm.qj` 拷进 `Keyboard/Data/`。

产品数据默认取仓库根的 `data/generated/`，没有就先跑 `tools/release/data-fetch.sh`。Rust 工具链要有 `aarch64-apple-ios` 与 `aarch64-apple-ios-sim` 两个 target。

装到真机：Xcode 登录了开发者账号的话，`DEVELOPMENT_TEAM=<团队 ID> xcodegen generate` 后直接 Run。
没登录时用 `scripts/install-device.sh`：它会编 Release、用本机的通配符描述文件手动签名，然后装到连着的 iPhone。

## 连青简 Cloud

把 `cloud.example.toml` 复制成 `cloud.local.toml`（不进仓库），填服务器地址与这台设备的令牌（服务器上 `qingjian-cloud device add iphone`），
重新构建即可；没有这个文件键盘完全离线。连上之后：

- **大模型补候选**：组字时上游的云联想（`CloudPredictor`）经服务器的 `/v1/chat/completions` 代理问模型，词与整句插在首选之后，用强调色；不抢首选，免得结果回来时空格上屏的字变了。
- **润色**：没在组字时候选栏是「✨ 润色」，把光标前这一段（从上一个换行起，最多 300 字）改通顺，结果点一下替换原文；光标前已经不是原文就不替换。
- **学习数据同步**：与 Mac 端同一套 `DataSync`（基线 + 增量 + 收件箱），基线在扩展容器的 `Library/Application Support/Qingjian/cloud/`。
  键盘出现、收起时各催一轮，可见期间每 0.25 秒合并一次收件箱。`config.toml` 与输入日志不同步（键盘不读前者、iOS 上不记后者）。

配置暂时在构建时打进键盘：主 App 与扩展共享设置要 App Group，而本机的通配符描述文件不支持。

## 启用

设置 → 通用 → 键盘 → 键盘 → 添加新键盘 → 青简；要震动就再点进「青简」打开「允许完全访问」。学习数据存在扩展自己的容器里（`Library/Application Support/Qingjian/user.tsv`），键盘收起时落盘。

模拟器里可以省掉在设置里点来点去：

```bash
xcrun simctl spawn booted defaults write .GlobalPreferences AppleKeyboards -array "app.qingjian.cloud.keyboard" "en_US@sw=QWERTY;hw=Automatic"
```

## 已知问题

- **⌄ 的短点击：** 模拟器里，候选栏最右侧约 48pt 宽的区域，按下不到约 0.2 秒的点击会被系统吞掉，送不到键盘（候选栏其他位置与所有按键都正常）。原因还没查清，要在真机上确认是否同样存在。
- **内存还没量：** 键盘扩展的上限约 48–60 MB；词库与语言模型走 mmap，常驻占用要在真机上用 Instruments 量过。
- **没打字时候选栏是空的：** 上屏后的联想还没接。
