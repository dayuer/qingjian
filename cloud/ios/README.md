# iOS 键盘

青简 Cloud 的 iOS 主 App 与键盘扩展。键盘里跑完整的本地引擎（`qingjian-cloud-bridge` 编成的静态库），不联网。
「完全访问」只用于按键震动（iOS 规定第三方键盘没有它不能震），不开照常打字、只有系统键盘音。

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
