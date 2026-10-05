# UI 测试与截图走查

`QingjianCloudUITests` 是给 UI 审计员对稿用的截图走查，加上几条端到端的流程（首页快速记、候选栏滑动、记一笔）。
截图走 XCUITest 的 attachment，跑完用 `xcrun xcresulttool export attachments --path <结果包> --output-path <目录>` 导出。
坐标都按 390×844 的屏量（iPhone 17e），换别的机型键盘那几条会点偏。

## 准备一台专用模拟器

```bash
xcrun simctl create Sujian-UI "iPhone 17e" com.apple.CoreSimulator.SimRuntime.iOS-26-5
xcrun simctl boot Sujian-UI
# 素笺键盘排第一（键盘那几条默认弹出来的就是它），系统拼音留着给 KeyboardCompare 对比；改完重启模拟器才生效
xcrun simctl spawn Sujian-UI defaults write .GlobalPreferences AppleKeyboards -array \
  "sujian.synon.ai.keyboard" "zh_Hans-Pinyin@sw=Pinyin-Simplified;hw=Automatic"
xcrun simctl shutdown Sujian-UI && xcrun simctl boot Sujian-UI
```

构建（`QINGJIAN_DATA` 指向产品数据）后先把 App 装上（`xcodebuild … build-for-testing` 之后 `xcrun simctl install`，或者先跑一次任意 UI 测试），
App Group 容器才存在，然后：

1. **开完全访问**：跑一次 `EnableFullAccess`（它驱动系统设置去打开开关）。键盘的记忆界面没有它出不来。
2. **种数据**：`UITests/seed/seed.py --device Sujian-UI`。只写合成数据：14 个人（名字是设计稿里的虚构角色，小美带代号「阿美」）、
   小美有「考试」「香菜」等卡片、键盘当前选着小美。每次都先清掉整个 `memory/` 再写。
   - `--legacy`：老格式（卡片文件是光秃秃的数组，没有 `{"rev","cards"}`）；
   - `--bad-card`：另给阿林塞一张关键词只有 1 个字的坏卡；
   - `--clean`：只清掉 `memory/`。
3. 深色另跑一遍：`xcrun simctl ui Sujian-UI appearance dark`，跑完改回 `light`。

## 各条要什么

| 测试 | 数据 | 键盘 |
| --- | --- | --- |
| `VisualWalkthrough`、`ContactsShots`、`HomeFlow`、`ReviewShots.testAppPages` | seed | 不用 |
| `KeyboardShots`、`KeyboardCompare`、`CandidateBarScroll`、`ReviewShots.testNoteFlow` | seed | 素笺键盘已加、完全访问已开 |

键盘那几条没加键盘时**不会失败**，只是弹出来的是系统键盘、截图没有意义，看截图才知道。
`ReviewShots.testNoteFlow` 每次换一句剪贴板（键盘记着处理过的那段，同一段第二次不出确认条），第一次读剪贴板时系统会弹「允许粘贴」，测试会点掉。
