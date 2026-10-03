# iOS 键盘

青简 Cloud 的 iOS 主 App 与键盘扩展。键盘里跑完整的本地引擎（`qingjian-cloud-bridge` 编成的静态库）。
「完全访问」用于按键震动（iOS 规定第三方键盘没有它不能震）与连青简 Cloud；不开照常离线打字、只有系统键盘音。

键位与交互照 iOS 26 自带的简体拼音 26 键：

- 三层键位：字母、`123`（数字与 `。，、？！` 等中文标点）、`#+=`；
- 拼音以 marked text 写在宿主的光标处；
- 空格上屏首选，换行原样上屏字母（打英文靠它）；
- 候选栏右端 ⌄ 展开全部候选；😀 打开自带的表情面板；
- 键与键之间没有死区：每个键的触摸范围延伸到缝隙中线，最外面的键到键区边缘（`KeyboardLayout`），点在缝里算离得近的键。

## 构建

```bash
brew install xcodegen
cd cloud/ios
xcodegen generate                 # 生成 QingjianCloud.xcodeproj（不进仓库）
open QingjianCloud.xcodeproj
```

Keyboard target 的 preBuildScript 会跑 `scripts/build-bridge.sh`，它做两件事：

- 编出真机加模拟器的 `Frameworks/QingjianBridge.xcframework`；
- 把 `dict.qj`、`lm.qj` 与领域词库 `dicts/*.qj` 拷进 `Keyboard/Data/`。

产品数据默认取仓库根的 `data/generated/`，没有就先跑 `tools/release/data-fetch.sh`。Rust 工具链要有 `aarch64-apple-ios` 与 `aarch64-apple-ios-sim` 两个 target。

装到真机：主 App 与键盘靠 App Group `group.app.qingjian.cloud` 共享设置，通配符描述文件不支持，必须自动签名——
先在 Xcode → Settings → Accounts 登录团队 L9YRXEKYN2 的账号，然后直接在 Xcode 里 Run，或跑 `scripts/install-device.sh`
（编 Release、`-allowProvisioningUpdates` 自动生成描述文件、装到连着的 iPhone，并检查键盘签名里带了 App Group）。

## 设置

主 App 首页有「键盘设置」与「青简 Cloud」两页，对应 Mac 偏好设置里键盘用得上的那部分：

- **键盘设置**：拼音方案（全拼、各家双拼；注音在 iPhone 上按全拼）、模糊音、繁体输出、全角标点、学习开关、领域词库、自定义短语。
  写的是与 Mac 同格式的 `config.toml`（经 `qingjian-platform` 逐项改，Mac 专属的设置与注释原样保留），开了同步时经 `DataSync` 与 Mac 互通。
  键盘每次轮询按修改时间重读，改完切回键盘就生效。
- **青简 Cloud**：只读显示服务器地址与是否已配置，可调的只有各项功能开关（同步、日志、剪贴板、润色、补候选）。
  服务器地址与令牌随安装配好，设置页改不了、令牌也不出桥；写回 `cloud.toml` 时只换开关。键盘下次弹出时发现文件变了就重开会话。

文件都在 App Group 容器的 `Library/Application Support/Qingjian/`。键盘开了完全访问时学习数据也放这里（第一次从扩展自己的容器搬过来）；
没开时 iOS 不许键盘写共享目录，学习数据留在扩展容器，只读共享目录里的设置。

## 连青简 Cloud

把 `cloud.example.toml` 复制成 `cloud.local.toml`（不进仓库），填服务器地址与这台设备的令牌（服务器上 `qingjian-cloud device add iphone`），
重新构建即可；没有这个文件键盘完全离线。连上之后：

- **大模型补候选**：组字时上游的云联想（`CloudPredictor`）经服务器的 `/v1/chat/completions` 代理问模型，词与整句插在首选之后，用强调色；不抢首选，免得结果回来时空格上屏的字变了。
- **润色**：没在组字时候选栏是「✨ 润色」，把光标前这一段（从上一个换行起，最多 300 字）改通顺，结果点一下替换原文；光标前已经不是原文就不替换。
- **跨设备剪贴板**：键盘弹出时拉一次服务器上的剪贴板事件，别的设备 10 分钟内复制的最新一条显示在候选栏（📋 macbook：…），点一下插入、✕ 不再提示。
  本机剪贴板变了（只看 `changeCount`，不读内容就不弹系统的粘贴授权提示）时给「发到其他设备」，点了才读剪贴板上传。进度在学习数据目录的 `cloud/clipboard.json`。
- **隐私输入框**：验证码、新密码、信用卡号这类字段（`textContentType`）切到引擎的私密输入——不学习、不记日志、不发云端，剪贴板与润色也停，候选栏亮一把锁。
  真正的密码框（`secureTextEntry`）系统根本不给第三方键盘。
- **学习数据同步**：与 Mac 端同一套 `DataSync`（基线 + 增量 + 收件箱），基线在学习数据目录的 `cloud/` 下。
  键盘出现、收起时各催一轮，可见期间每 0.25 秒合并一次收件箱。`config.toml` 也一起同步；别的设备的输入日志不下载。

`cloud.local.toml` 构建时打进键盘，只作种子：主 App 第一次打开时拷进 App Group，之后以设置页里改的为准。

## 启用

设置 → 通用 → 键盘 → 键盘 → 添加新键盘 → 青简；要震动就再点进「青简」打开「允许完全访问」。学习数据的位置见上面「设置」一节，键盘收起时落盘。

模拟器里可以省掉在设置里点来点去：

```bash
xcrun simctl spawn booted defaults write .GlobalPreferences AppleKeyboards -array "app.qingjian.cloud.keyboard" "en_US@sw=QWERTY;hw=Automatic"
```

## 已知问题

- **⌄ 的短点击：** 模拟器里，候选栏最右侧约 48pt 宽的区域，按下不到约 0.2 秒的点击会被系统吞掉，送不到键盘（候选栏其他位置与所有按键都正常）。原因还没查清，要在真机上确认是否同样存在。
- **内存还没量：** 键盘扩展的上限约 48–60 MB；词库与语言模型走 mmap，常驻占用要在真机上用 Instruments 量过。
- **没打字时候选栏是空的：** 上屏后的联想还没接。
