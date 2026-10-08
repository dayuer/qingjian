# 素笺 App 四页改成设计稿的白页 + 自绘分组 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 对象详情（02 的 1b）、改一条（1c）、对象设置（1d）、「我」（02 的 2c / 05 的 2j）四页从系统 `Form` / `List` 改成设计稿的样子：白纸底、自绘的 `.group` 分组、`.sec-t` 小节标题；「我」补上「懒得自己写？」与「数据」一组。

**Architecture:** 新建 `App/Paper/` 一组小件（页、分组、行、细线、小节标题、输入框），照 `cloud/design/mockups/theme.css` 的 `.app` `.scroll` `.group` `.row` `.mem` `.sec-t` `.lbl` `.field` `.lead` `.btn` 取值；四页改用它们，逻辑（保存、确认、绑定）不动。「我」的数据一组复用已有的桥调用（`AccountStore.setConsent` / `deleteAccount`、`MemoryStore.exportText`），只加一个导出全部与一个撤回全部的纯函数。

**Tech Stack:** Swift 6 / SwiftUI（部署目标 iOS 17：**不能用** `Group(subviews:)`、`ForEach(subviews:)` 这类 iOS 18 API），XcodeGen，XCTest / XCUITest。

**前置：** 在 PR dayuer/qingjian#9（分支 `fix/ui-align-design`）之上做：它加了 `Shared/Hairline.swift`、`Theme.ink2Tone`、`Theme.paper2`、`ColorRole.ink3`、`ColorUsage.editorConfirm / editorCancel / exportLink` 等，本计划直接用。

**约定（`docs/contributing.md` 的 Swift 版）：** 一个类型一个文件、新文件头两行注释说它是什么；注释中文、只写约束与原因；颜色一律经 `ColorUsage` / `Theme` / `Hairline` 取，不在视图里写色值；不用系统红（设计稿 `.btn.danger` 是 ink，只有 2j 的两行危险操作是 `oklch(0.5 0.13 28)`）。提交信息 `fix(cloud): …` / `feat(cloud): …`，中文说明，**不写 Co-Authored-By 之类的署名尾行**。

---

## 设计稿取值速查（theme.css）

| 类 | 取值 | 本计划里对应 |
|---|---|---|
| `.app` | 底 `--paper` oklch(0.99) = #FCFCFC | `Theme.paper` |
| `.scroll` | 竖排、`gap:18px`、`padding:12px 0 20px`；2j 用 `gap:10px` + 行 `padding:10px 14px`（data-tight） | `PaperPage(spacing:)`、`PaperRow(tight:)` |
| `.sec-t` | `500 12px`、ink-3、左右 20、`letter-spacing:.5px` | `PaperSectionTitle` |
| `.group` | 左右外边距 16、白底、圆角 14、外圈 1px oklch(0.92) | `PaperGroup`（外圈 `Hairline.ring`） |
| `.row` | 横排、`gap:12px`、`padding:12px 14px`、行间 1px oklch(0.94) | `PaperRow` + `PaperRowLine`（`Hairline.row`） |
| `.row .ttl` / `.sub` / `.side` / `.chev` | 15px / 12.5px ink-3 行高 1.45 / 12px ink-3 / 16px ink-3「›」 | `PaperRow` 的四个参数 |
| `.mem` | 竖排、`gap:4px`、`padding:12px 14px`；`.tx` 15px 行高 1.5；`.meta` 11.5px ink-3 | `PaperMemRow` |
| `.lbl` | `500 12px` ink-3 | `PaperLabel` |
| `.field` | 白底、圆角 12、外圈 1px `--line`、`padding:12px 14px`、16px 行高 1.5 | `PaperFieldStyle` |
| `.lead` | 13.5px 行高 1.6、ink-2、左右 20（1d 那句是 12px ink-3） | `PaperLead` |
| `.btn.danger` | 13px 500、ink | 「删掉这条」「忘掉这个人」 |
| `.nav .back` | accent-ink | 系统返回键 tint（已经是 `appLink` = accentInk） |
| 2j 危险行 | `.ttl` 颜色 oklch(0.5 0.13 28) | `Theme.danger`（浅 #A04037，深 oklch(0.72 0.13 28) #EB8376） |

深色：设计稿只有浅色，照 `Theme.swift` 的做法按明暗对调取（纸 oklch(0.2) #161616，分组底 oklch(0.24) #1F1F1F）。

---

## 文件结构

| 路径 | 职责 |
|---|---|
| `cloud/ios/Shared/Theme.swift` | 加 `paperTone` / `paper`、`paperCardTone` / `paperCard`、`danger` |
| `cloud/ios/App/Paper/PaperPage.swift` | 白纸底的竖向滚动页（`.app` + `.scroll`），留出浮动 Tab 栏 |
| `cloud/ios/App/Paper/PaperGroup.swift` | 白底圆角分组（`.group`） |
| `cloud/ios/App/Paper/PaperRow.swift` | 一行：标题 / 副标题 / 右侧字 / 右侧件 / ›（`.row`） |
| `cloud/ios/App/Paper/PaperMemRow.swift` | 一条记忆（`.mem`） |
| `cloud/ios/App/Paper/PaperRowLine.swift` | 行间细线 |
| `cloud/ios/App/Paper/PaperSectionTitle.swift` | 小节标题（`.sec-t`） |
| `cloud/ios/App/Paper/PaperLabel.swift` | 表单小标签（`.lbl`） |
| `cloud/ios/App/Paper/PaperLead.swift` | 说明段（`.lead`） |
| `cloud/ios/App/Paper/PaperFieldStyle.swift` | 输入框样式（`.field`） |
| `cloud/ios/App/Remember/RememberView.swift` | 私有的 `group` / `sectionTitle` / `rowLine` 换成上面的件（DRY） |
| `cloud/ios/App/Memory/ContactDetailView.swift` | 1b 改写 |
| `cloud/ios/App/Memory/MaterialsSection.swift`、`MaterialRow.swift` | 跟着离开 `List`（左滑删除去掉，展开后的「删除」保留） |
| `cloud/ios/App/Memory/CardEditor.swift` | 1c 改写（底部弹层 + 自绘表单） |
| `cloud/ios/App/Memory/KeywordChips.swift` | 1c 的关键词一节（一个类型一个文件，CardEditor 已经 177 行） |
| `cloud/ios/App/Memory/ContactSettingsView.swift` | 1d 改写 |
| `cloud/ios/App/Memory/MemoryStore.swift` | 加 `exportAllText()` |
| `cloud/ios/App/Me/MeView.swift` | 2c / 2j 改写 |
| `cloud/ios/App/Me/MeDataSection.swift` | 「数据」一组（免费 / 开通两种） |
| `cloud/ios/App/Me/CloudPitchCard.swift` | 「懒得自己写？」卡 |
| `cloud/ios/App/Me/ProcessorsView.swift` | 「交给谁处理」页 |
| `cloud/ios/App/Me/MeWording.swift` | 「我」页这几处文案与纯函数（可测） |
| `cloud/ios/Tests/ThemeTests.swift`、`MemoryStoreTests.swift`、新 `MeWordingTests.swift` | 单测 |
| `cloud/docs/plans/2026-10-05-ui-implementation.md` | 清单状态与约束 7 同步 |

---

## 构建与测试命令（每个任务都用）

模拟器用 iPhone 17e（`xcrun simctl list devices available | grep "iPhone 17e"` 拿 UDID，下文记作 `$D`）。

```bash
cd cloud/ios && xcodegen generate -q
xcodebuild test -project QingjianCloud.xcodeproj -scheme QingjianCloud \
  -destination "id=$D" -only-testing:QingjianCloudTests CODE_SIGNING_ALLOWED=NO 2>&1 \
  | grep -E "error:|with [1-9][0-9]* failures|\*\* TEST"
```

预期最后一行 `** TEST SUCCEEDED **`。第一次会编 Rust 桥，几分钟。

---

### Task 0：开分支

- [ ] **Step 1：从 PR #9 的分支开新分支**

```bash
cd /Users/liyuqing/sproot/qingjian
git fetch origin
git switch -c feat/ui-paper-pages origin/fix/ui-align-design
```

- [ ] **Step 2：确认基线测试是绿的**（上面的测试命令），记下耗时。

---

### Task 1：纸、分组底、危险色三个 token

**Files:** Modify `cloud/ios/Shared/Theme.swift`；Test `cloud/ios/Tests/ThemeTests.swift`

- [ ] **Step 1：写失败的测试**——`ThemeTests` 的 `swatches` 加三项，另加一条值的断言：

```swift
    private var swatches: [ThemeSwatch] {
        [Theme.accent, Theme.accentInk, Theme.accentSoft, Theme.hintLine, Theme.ink2Tone,
         Theme.paperTone, Theme.paperCardTone, Theme.danger].flatMap { [$0.light, $0.dark] }
    }

    func testPaperAndDangerHexValues() {
        XCTAssertEqual(Theme.paperTone.light.hex, 0xFCFCFC)
        XCTAssertEqual(Theme.paperCardTone.light.hex, 0xFFFFFF)
        XCTAssertEqual(Theme.danger.light.hex, 0xA04037)
        XCTAssertEqual(Theme.danger.dark.hex, 0xEB8376)
    }
```

- [ ] **Step 2：跑测试，预期编译失败**（`Theme` 没有 `paperTone`）。

- [ ] **Step 3：实现**——`Theme.swift` 里 `paper2` 前面加：

```swift
    /// 页面底（设计稿 --paper oklch(0.99 0 0)），深色对调成 oklch(0.2)。
    static let paperTone = ThemeColor(
        light: ThemeSwatch(hex: 0xFCFCFC, l: 0.99, c: 0, h: 0),
        dark: ThemeSwatch(hex: 0x161616, l: 0.2, c: 0, h: 0))

    static let paper = paperTone.color

    /// 分组卡片的底（设计稿 .group 白底），深色 oklch(0.24)。
    static let paperCardTone = ThemeColor(
        light: ThemeSwatch(hex: 0xFFFFFF, l: 1, c: 0, h: 0),
        dark: ThemeSwatch(hex: 0x1F1F1F, l: 0.24, c: 0, h: 0))

    static let paperCard = paperCardTone.color

    /// 「我」里两行危险操作的字（设计稿 2j：oklch(0.5 0.13 28)），深色 oklch(0.72 0.13 28)。别处不用红。
    static let danger = ThemeColor(
        light: ThemeSwatch(hex: 0xA04037, l: 0.5, c: 0.13, h: 28),
        dark: ThemeSwatch(hex: 0xEB8376, l: 0.72, c: 0.13, h: 28))
```

- [ ] **Step 4：跑测试，预期全过。** 若 `testSwatchesMatchOKLCHSources` 对某个十六进制差 1 以上，以 `OKLCH.srgb` 算出来的为准改十六进制（测试是尺子）。

- [ ] **Step 5：提交**

```bash
git add cloud/ios/Shared/Theme.swift cloud/ios/Tests/ThemeTests.swift
git commit -m "feat(cloud): 加设计稿的纸色、分组底与危险色"
```

---

### Task 2：`App/Paper/` 一组件

**Files:** Create 下列 9 个文件（目录 `cloud/ios/App/Paper/`，XcodeGen 按目录收，不用改 project.yml）。没有逻辑，不写单测；Task 3 起用它们时靠截图验。

- [ ] **Step 1：`PaperPage.swift`**

```swift
// 设计稿 .app + .scroll：白纸底、竖向滚动、各块之间等距（默认 18，「我」用 10）。
// 底部留出 iOS 26 浮动 Tab 栏的高度（RootTab.listBottomMargin），不让最后一块被挡。

import SwiftUI

struct PaperPage<Content: View>: View {
    var spacing: CGFloat = 18

    @ViewBuilder let content: Content

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: spacing) {
                content
            }
            .padding(.top, 12)
            .padding(.bottom, 20)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .contentMargins(.bottom, RootTab.listBottomMargin, for: .scrollContent)
        .background(Theme.paper)
    }
}
```

- [ ] **Step 2：`PaperGroup.swift`**

```swift
// 设计稿 .group：左右外边距 16、白底、圆角 14、外圈一道淡灰细线。行与行之间的细线由调用方放 PaperRowLine
// （iOS 17 没有 Group(subviews:)，取不到子视图挨个插线）。fill 可换成 paper2（2c 的「懒得自己写？」是灰底、无外圈）。

import SwiftUI

struct PaperGroup<Content: View>: View {
    var fill: Color = Theme.paperCard

    var ring = true

    @ViewBuilder let content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            content
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(RoundedRectangle(cornerRadius: 14).fill(fill))
        .overlay {
            if ring { RoundedRectangle(cornerRadius: 14).strokeBorder(Hairline.ring) }
        }
        .clipShape(RoundedRectangle(cornerRadius: 14))
        .padding(.horizontal, 16)
    }
}
```

- [ ] **Step 3：`PaperRowLine.swift`**

```swift
// 设计稿 .row / .mem 的下沿：分组里行与行之间 1pt 的淡灰线，通栏（不缩进）。

import SwiftUI

struct PaperRowLine: View {
    var body: some View {
        Rectangle().fill(Hairline.row).frame(height: 1)
    }
}
```

- [ ] **Step 4：`PaperRow.swift`**

```swift
// 设计稿 .row：左边标题（15）与可选副标题（12.5 ink-3），右边可选的灰字（.side 12 ink-3）或任意控件，
// 最后可选的「›」（.chev）。tight 是 2j 那种紧凑行（上下 10）。titleColor 只有「我」的两行危险操作会改。

import SwiftUI

struct PaperRow<Trailing: View>: View {
    let title: String

    var subtitle: String?

    var side: String?

    var chevron = false

    var tight = false

    var titleColor: Color = Theme.ink

    @ViewBuilder var trailing: Trailing

    var body: some View {
        HStack(spacing: 12) {
            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                    .font(AppFont.font(size: 15))
                    .foregroundStyle(titleColor)
                if let subtitle {
                    Text(subtitle)
                        .font(AppFont.font(size: 12.5))
                        .foregroundStyle(Theme.ink3)
                        .lineSpacing(2)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            if let side {
                Text(side)
                    .font(AppFont.font(size: 12))
                    .foregroundStyle(Theme.ink3)
                    .multilineTextAlignment(.trailing)
            }
            trailing
            if chevron {
                Text("›")
                    .font(AppFont.font(size: 16))
                    .foregroundStyle(Theme.ink3)
            }
        }
        .padding(.horizontal, 14)
        .padding(.vertical, tight ? 10 : 12)
        .contentShape(Rectangle())
    }
}

extension PaperRow where Trailing == EmptyView {
    init(
        title: String, subtitle: String? = nil, side: String? = nil, chevron: Bool = false, tight: Bool = false,
        titleColor: Color = Theme.ink
    ) {
        self.init(
            title: title, subtitle: subtitle, side: side, chevron: chevron, tight: tight, titleColor: titleColor
        ) { EmptyView() }
    }
}
```

- [ ] **Step 5：`PaperMemRow.swift`**

```swift
// 设计稿 .mem：一条记忆的正文（15，行高 1.5），下面可选的 meta 小字（11.5 ink-3）。对象详情里没日子的卡用它。

import SwiftUI

struct PaperMemRow: View {
    let text: String

    var meta: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(text)
                .font(AppFont.font(size: 15))
                .lineSpacing(7.5)
                .foregroundStyle(Theme.ink)
                .multilineTextAlignment(.leading)
            if let meta {
                Text(meta)
                    .font(AppFont.font(size: 11.5))
                    .foregroundStyle(Theme.ink3)
            }
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .contentShape(Rectangle())
    }
}
```

- [ ] **Step 6：`PaperSectionTitle.swift`**

```swift
// 设计稿 .sec-t：分组上面的小节标题，500 12 ink-3、字距 0.5、左右 20；右边可选一段常规字重的说明。

import SwiftUI

struct PaperSectionTitle: View {
    let text: String

    var trailing: String?

    var body: some View {
        HStack {
            Text(text)
            Spacer()
            if let trailing {
                Text(trailing).fontWeight(.regular)
            }
        }
        .font(AppFont.font(size: 12, weight: .medium))
        .tracking(0.5)
        .foregroundStyle(Theme.ink3)
        .padding(.horizontal, 20)
    }
}
```

- [ ] **Step 7：`PaperLabel.swift`、`PaperLead.swift`、`PaperFieldStyle.swift`**

```swift
// 设计稿 .lbl：表单里输入框上面的小标签，500 12 ink-3。

import SwiftUI

struct PaperLabel: View {
    let text: String

    var body: some View {
        Text(text)
            .font(AppFont.font(size: 12, weight: .medium))
            .foregroundStyle(Theme.ink3)
    }
}
```

```swift
// 设计稿 .lead：分组下面的说明段。缺省 13.5 ink-2 行高 1.6；1d 名字下那句是 12 ink-3（small）。左右 20。

import SwiftUI

struct PaperLead: View {
    let text: String

    var small = false

    var body: some View {
        Text(text)
            .font(AppFont.font(size: small ? 12 : 13.5))
            .lineSpacing(small ? 5 : 8)
            .foregroundStyle(small ? Theme.ink3 : Theme.ink2)
            .padding(.horizontal, 20)
            .frame(maxWidth: .infinity, alignment: .leading)
    }
}
```

```swift
// 设计稿 .field：白底、圆角 12、一圈 --line 细线、内边距 12 / 14、16pt 字。改一条的「记忆」框用它。

import SwiftUI

struct PaperFieldStyle: ViewModifier {
    func body(content: Content) -> some View {
        content
            .font(AppFont.font(size: 16))
            .padding(.horizontal, 14)
            .padding(.vertical, 12)
            .background(RoundedRectangle(cornerRadius: 12).fill(Theme.paperCard))
            .overlay(RoundedRectangle(cornerRadius: 12).strokeBorder(Hairline.line))
    }
}
```

- [ ] **Step 8：首页改用这些件（DRY）**——`App/Remember/RememberView.swift` 里：
  - `sectionTitle(_:trailing:)` 的三处调用换成 `PaperSectionTitle(text:trailing:)`，删掉私有的 `sectionTitle`；
  - 私有的 `group { … }` 换成 `PaperGroup { … }`（`fill` 保持缺省），删掉私有的 `group`；
  - 私有的 `rowLine` 换成 `PaperRowLine()`，删掉私有的 `rowLine`；
  - 功勋路卡片的 `.background(RoundedRectangle…secondarySystemGroupedBackground…)` 那段换成外面包一层 `PaperGroup`（里面的内边距保留）。

- [ ] **Step 9：构建并跑单测**（命令同上），预期全过。

- [ ] **Step 10：提交**

```bash
git add cloud/ios/App/Paper cloud/ios/App/Remember/RememberView.swift
git commit -m "feat(cloud): 照设计稿 theme.css 抽出白页、分组、行等一组件，首页改用"
```

---

### Task 3：导出全部记忆（给「我」用）

**Files:** Modify `cloud/ios/App/Memory/MemoryStore.swift`（在 `exportText(_:now:)` 后面）；Test `cloud/ios/Tests/MemoryStoreTests.swift`

- [ ] **Step 1：写失败的测试**——照 `MemoryStoreTests` 里已有的 `exportText` 测试的种数据写法（先读那条测试，用同一个 helper 建两个人各一张卡），加：

```swift
    func testExportAllJoinsEveryContactInListOrder() async throws {
        let store = try await seededStore(contacts: ["小美", "阿林"])   // 用文件里现成的种数据 helper；名字不同就照它改
        let all = store.exportAllText(now: now)
        let parts = store.contacts.map { store.exportText($0.id, now: now) }
        XCTAssertEqual(all, parts.joined(separator: "\n\n"))
        XCTAssertTrue(all.contains("小美"))
        XCTAssertTrue(all.contains("阿林"))
    }

    func testExportAllIsEmptyWithoutContacts() async throws {
        let store = try await seededStore(contacts: [])
        XCTAssertEqual(store.exportAllText(now: now), "")
    }
```

（如果文件里没有 `seededStore` 这样的 helper，就照该文件 `exportText` 测试的建法在测试里内联建 store，不要新造测试基础设施。）

- [ ] **Step 2：跑，预期编译失败**（没有 `exportAllText`）。

- [ ] **Step 3：实现**

```swift
    /// 「我 → 导出记忆卡」：每个人一段（同 `exportText`），按名单顺序空一行接起来；没有人时是空串。
    func exportAllText(now: Date = Date()) -> String {
        contacts.map { exportText($0.id, now: now) }.joined(separator: "\n\n")
    }
```

（名单属性若不叫 `contacts`，用 `MemoryStore` 里实际的名单属性名。）

- [ ] **Step 4：跑，预期全过。**

- [ ] **Step 5：提交**

```bash
git add cloud/ios/App/Memory/MemoryStore.swift cloud/ios/Tests/MemoryStoreTests.swift
git commit -m "feat(cloud): 记忆卡可以一次导出全部人"
```

---

### Task 4：「我」页的文案与纯函数

**Files:** Create `cloud/ios/App/Me/MeWording.swift`；Test Create `cloud/ios/Tests/MeWordingTests.swift`（`QingjianCloudTests` 的 sources 是整个 `Tests/` 加 `@testable import QingjianCloud`，App 里的类型直接可见）

文案定稿（设计稿 2c / 2j 去掉场景与账号后的版本）：

| 处 | 文案 |
|---|---|
| 懒得自己写？卡标题 | 懒得自己写？ |
| 卡正文 | 开通素笺云服务后，键盘会在聊天里自己记、每天整理，你只用每周确认一下。 |
| 卡链接 | 了解一下 |
| 免费版导出行 | 导出记忆卡 / 副标题「都是你写的 · 只存在这台手机上」 |
| 开通后导出行 | 导出我的记忆 |
| 交给谁处理 | 交给谁处理 / 副标题「腾讯云 · DeepSeek · 已同意 n 项」，一项都没开写「还没开启云功能」 |
| 撤回 | 撤回同意并删除云端数据 / 副标题「手机上的记忆卡留着」 |
| 撤回确认 | 标题「撤回同意？」；正文「会关掉全部云功能，服务器上这些功能的数据一并删除。手机上的记忆卡留着。」；按钮「撤回并删除」「取消」 |
| 删除空间 | 删除素笺云服务 / 副标题「所有设备退出，服务器上的数据全部删除」 |
| 删除确认 | 标题「删除素笺云服务？」；正文「所有设备都会退出，服务器上的数据全部删除，无法恢复。手机上的记忆卡留着。」；按钮「删除」「取消」 |

（「删除素笺云服务」替代设计稿的「删除账号」——约束 5：界面不出现「账号」。这条文案在 UI 清单里标「待审计会话确认」。）

- [ ] **Step 1：写失败的测试**

```swift
// 「我」页的文案与撤回同意要关哪几项。

import XCTest
@testable import QingjianCloud

final class MeWordingTests: XCTestCase {
    func testProcessorsSubtitleCountsGrantedFeatures() {
        var consents = Consents()
        XCTAssertEqual(MeWording.processorsSubtitle(consents), "还没开启云功能")
        consents[.memory] = true
        consents[.llm] = true
        XCTAssertEqual(MeWording.processorsSubtitle(consents), "腾讯云 · DeepSeek · 已同意 2 项")
    }

    func testWithdrawTurnsOffOnlyWhatIsOn() {
        var consents = Consents()
        consents[.clipboard] = true
        consents[.memory] = true
        XCTAssertEqual(MeWording.featuresToWithdraw(consents), [.clipboard, .memory])
        XCTAssertEqual(MeWording.featuresToWithdraw(Consents()), [])
    }

    func testCopy() {
        XCTAssertEqual(MeWording.pitchTitle, "懒得自己写？")
        XCTAssertEqual(MeWording.exportFree, "导出记忆卡")
        XCTAssertEqual(MeWording.exportFreeNote, "都是你写的 · 只存在这台手机上")
        XCTAssertEqual(MeWording.withdrawTitle, "撤回同意并删除云端数据")
        XCTAssertEqual(MeWording.deleteSpaceTitle, "删除素笺云服务")
    }
}
```

先读 `App/Account/Consents.swift`：`Consents` 怎么构造、怎么按 `CloudFeature` 下标读写。上面的 `Consents()` 与 `consents[.memory] = true` 照它的实际 API 改（`SpaceEntryView` 里用的是 `account.state?.consents[feature]`）。

- [ ] **Step 2：跑，预期编译失败。**

- [ ] **Step 3：实现 `MeWording.swift`**

```swift
// 「我」页（02 的 2c、05 的 2j）的文案，以及撤回同意要关哪几项：纯值，单测见 MeWordingTests。
// 设计稿的场景一组已作废、「删除账号」换成「删除素笺云服务」（不出现「账号」，UI 清单约束 5）。

enum MeWording {
    static let pitchTitle = "懒得自己写？"
    static let pitchBody = "开通素笺云服务后，键盘会在聊天里自己记、每天整理，你只用每周确认一下。"
    static let pitchLink = "了解一下"

    static let dataTitle = "数据"
    static let exportFree = "导出记忆卡"
    static let exportFreeNote = "都是你写的 · 只存在这台手机上"
    static let exportCloud = "导出我的记忆"

    static let processorsTitle = "交给谁处理"

    static let withdrawTitle = "撤回同意并删除云端数据"
    static let withdrawNote = "手机上的记忆卡留着"
    static let withdrawConfirmTitle = "撤回同意？"
    static let withdrawConfirmBody = "会关掉全部云功能，服务器上这些功能的数据一并删除。手机上的记忆卡留着。"
    static let withdrawConfirmButton = "撤回并删除"

    static let deleteSpaceTitle = "删除素笺云服务"
    static let deleteSpaceNote = "所有设备退出，服务器上的数据全部删除"
    static let deleteSpaceConfirmTitle = "删除素笺云服务？"
    static let deleteSpaceConfirmBody = "所有设备都会退出，服务器上的数据全部删除，无法恢复。手机上的记忆卡留着。"
    static let deleteSpaceConfirmButton = "删除"

    /// 「交给谁处理」那一行的副标题：开了几项云功能。
    static func processorsSubtitle(_ consents: Consents) -> String {
        let count = CloudFeature.allCases.filter { consents[$0] }.count
        return count == 0 ? "还没开启云功能" : "腾讯云 · DeepSeek · 已同意 \(count) 项"
    }

    /// 撤回同意：把开着的逐项关掉（每关一项服务器删那部分数据），按 CloudFeature 的顺序。
    static func featuresToWithdraw(_ consents: Consents) -> [CloudFeature] {
        CloudFeature.allCases.filter { consents[$0] }
    }
}
```

- [ ] **Step 4：跑，预期全过。**

- [ ] **Step 5：提交**

```bash
git add cloud/ios/App/Me/MeWording.swift cloud/ios/Tests/MeWordingTests.swift
git commit -m "feat(cloud): 「我」页数据一组的文案与撤回要关的项"
```

---

### Task 5：「交给谁处理」页

**Files:** Create `cloud/ios/App/Me/ProcessorsView.swift`

内容只陈述 `Account/ConsentSheet.swift` 里已经对用户说过的事，不新增承诺。先读 `ConsentSheet.swift` 的两份文案，下面的句子若和它冲突，以它为准改。

- [ ] **Step 1：实现**

```swift
// 「我 → 交给谁处理」（05 的 2j）：谁存、谁处理你开启的云功能的数据，以及每项现在开没开。
// 只复述同意页（Account/ConsentSheet）已经说过的，开关仍在「素笺云服务」页改，这里只读。

import SwiftUI

struct ProcessorsView: View {
    let consents: Consents

    var body: some View {
        PaperPage {
            PaperSectionTitle(text: "存放")
            PaperGroup {
                PaperRow(title: "腾讯云（新加坡）", subtitle: "存放你开启的云功能的数据，并在那里处理同步与整理。")
            }
            PaperSectionTitle(text: "大模型")
            PaperGroup {
                PaperRow(
                    title: "DeepSeek（数据在中国境内处理）",
                    subtitle: "整理云端记忆、改写与云联想时使用；输入日志用于 AI 优化时先脱敏。DeepSeek 可能保存数据或用于改进它的模型，具体见它官网的隐私政策。")
            }
            PaperSectionTitle(text: "你开启的")
            PaperGroup {
                ForEach(Array(CloudFeature.allCases.enumerated()), id: \.element) { index, feature in
                    if index > 0 { PaperRowLine() }
                    PaperRow(title: feature.title, side: consents[feature] ? "已同意" : "没开启")
                }
            }
            PaperLead(text: "要改在「素笺云服务」里关掉对应的开关；关掉会同时删除服务器上这部分数据。", small: true)
        }
        .navigationTitle(MeWording.processorsTitle)
        .navigationBarTitleDisplayMode(.inline)
    }
}
```

- [ ] **Step 2：构建通过**（单测命令）。

- [ ] **Step 3：提交**

```bash
git add cloud/ios/App/Me/ProcessorsView.swift
git commit -m "feat(cloud): 「我」加「交给谁处理」页"
```

---

### Task 6：「我」改写（2c / 2j）

**Files:** Modify `cloud/ios/App/Me/MeView.swift`；Create `cloud/ios/App/Me/CloudPitchCard.swift`、`cloud/ios/App/Me/MeDataSection.swift`

页面从上到下（用 `PaperPage(spacing: 10)`，行都 `tight: true`）：

1. `PageHeader(title: "我")`（不动，放在 `PaperPage` 外面，整页底色 `Theme.paper`）
2. **没开通**：`CloudPitchCard`（点「了解一下」进 `SpaceEntryView(store: space)`，它在没开通时就是开通页）。**开通了**：一组一行「素笺云服务 / 已开通」带 ›，进 `SpaceEntryView`。
3. `PaperSectionTitle("记忆")` + 一组：一行说明（`ScopeDisplay.fullAccessExplanation`，15，用 `PaperMemRow`）、一行路径（`PaperMemRow(text: ScopeDisplay.fullAccessPath)` 换 12.5 ink-3：直接用 `PaperRow(title:)` 不合适，写成 `PaperMemRow` 之外的小 Text 也行）、一行「去开启」（`appLink` 色，点了 `UIApplication.openSettingsURLString`）。设计稿没有这组，有意保留（约束 2/3：记忆要完全访问，App 是唯一能跳设置的地方）。
4. `PaperSectionTitle("设置")` + 一组：「键盘设置」›、「关于」›（没 App Group 时第一行换成那句说明，同现在）。
5. `MeDataSection`（见下）。
6. 「重新看引导」：footnote、ink-3、居中，不在分组里（同现在）。

- [ ] **Step 1：`CloudPitchCard.swift`**

```swift
// 「我」顶上的「懒得自己写？」（02 的 2c）：只在没开通素笺云时出，灰底、无外圈；「了解一下」进开通页。

import SwiftUI

struct CloudPitchCard<Destination: View>: View {
    @ViewBuilder let destination: Destination

    var body: some View {
        PaperGroup(fill: Theme.paper2, ring: false) {
            NavigationLink {
                destination
            } label: {
                VStack(alignment: .leading, spacing: 2) {
                    Text(MeWording.pitchTitle)
                        .font(AppFont.font(size: 15, weight: .medium))
                        .foregroundStyle(Theme.ink)
                    Text(MeWording.pitchBody)
                        .font(AppFont.font(size: 12.5))
                        .lineSpacing(7)
                        .foregroundStyle(Theme.ink3)
                        .multilineTextAlignment(.leading)
                    Text(MeWording.pitchLink)
                        .font(AppFont.font(size: 13, weight: .medium))
                        .foregroundStyle(ColorUsage.appLink.role.color)
                        .padding(.top, 6)
                }
                .padding(.horizontal, 14)
                .padding(.vertical, 12)
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .buttonStyle(.plain)
        }
    }
}
```

- [ ] **Step 2：`MeDataSection.swift`**

```swift
// 「我」的数据一组（02 的 2c / 05 的 2j）。没开通：只有「导出记忆卡」。开通了：交给谁处理、导出我的记忆、
// 撤回同意并删除云端数据、删除素笺云服务；后两行用 Theme.danger，点了先系统确认框确认一次（alert：iOS 26 上
// confirmationDialog 是气泡、「取消」不显示，同「忘掉」）。

import SwiftUI

struct MeDataSection: View {
    let signedIn: Bool

    let memory: MemoryStore

    let account: AccountStore

    /// 删完空间后让「我」重读开通状态。
    let onSpaceDeleted: () -> Void

    @State private var confirmingWithdraw = false

    @State private var confirmingDelete = false

    var body: some View {
        PaperSectionTitle(text: MeWording.dataTitle)
        PaperGroup {
            if signedIn {
                NavigationLink {
                    ProcessorsView(consents: account.state?.consents ?? Consents())
                } label: {
                    PaperRow(
                        title: MeWording.processorsTitle,
                        subtitle: MeWording.processorsSubtitle(account.state?.consents ?? Consents()),
                        chevron: true, tight: true)
                }
                .buttonStyle(.plain)
                PaperRowLine()
                exportRow(title: MeWording.exportCloud, note: nil)
                PaperRowLine()
                Button { confirmingWithdraw = true } label: {
                    PaperRow(
                        title: MeWording.withdrawTitle, subtitle: MeWording.withdrawNote, tight: true,
                        titleColor: Theme.danger.color)
                }
                .buttonStyle(.plain)
                PaperRowLine()
                Button { confirmingDelete = true } label: {
                    PaperRow(
                        title: MeWording.deleteSpaceTitle, subtitle: MeWording.deleteSpaceNote, tight: true,
                        titleColor: Theme.danger.color)
                }
                .buttonStyle(.plain)
            } else {
                exportRow(title: MeWording.exportFree, note: MeWording.exportFreeNote)
            }
        }
        .alert(MeWording.withdrawConfirmTitle, isPresented: $confirmingWithdraw) {
            Button(MeWording.withdrawConfirmButton, role: .destructive) { Task { await withdraw() } }
            Button("取消", role: .cancel) {}
        } message: {
            Text(MeWording.withdrawConfirmBody)
        }
        .alert(MeWording.deleteSpaceConfirmTitle, isPresented: $confirmingDelete) {
            Button(MeWording.deleteSpaceConfirmButton, role: .destructive) {
                Task {
                    await account.deleteAccount()
                    onSpaceDeleted()
                }
            }
            Button("取消", role: .cancel) {}
        } message: {
            Text(MeWording.deleteSpaceConfirmBody)
        }
    }

    private func exportRow(title: String, note: String?) -> some View {
        ShareLink(item: memory.exportAllText()) {
            PaperRow(title: title, subtitle: note, chevron: true, tight: true)
        }
        .buttonStyle(.plain)
    }

    private func withdraw() async {
        guard let consents = account.state?.consents else { return }
        for feature in MeWording.featuresToWithdraw(consents) {
            await account.setConsent(feature, false)
        }
    }
}
```

**先核对两件事再往下写**（不清楚就停下来问，不要猜）：
  - `AccountStore.deleteAccount()` 调的桥 `qj_account_delete` 在「不要账号」之后是不是删整个空间、本机退出（读 `cloud/crates/qingjian-cloud-bridge` 里它的实现与 `cloud/docs/plans/2026-10-05-no-account-client.md`）。如果不是（例如只删身份），这一行先不放，在 UI 清单里记「删除空间待 1b」。
  - `deleteAccount()` 成功后它的 `message` 是「账号已删除，服务器上的数据已清除」——带了「账号」，改成「已删除素笺云服务，服务器上的数据已清除」（`AccountStore.swift`，同一个提交）。

- [ ] **Step 3：改 `MeView.swift`**——要点：
  - 加 `@State private var memory = MemoryStore()`、`@State private var account = AccountStore()`（先看 `RememberView` / `ContactsView` 怎么拿 `MemoryStore`：如果是从 `RootView` 传进来的，就同样由 `RootView` 传给 `MeView`，不要另建一份）；
  - `.task { space.refresh(); await account.refresh(); await memory.reload() }`（方法名照两个 store 的实际 API）；
  - 结构照上面 1–6；`Form` 与 `.background(Color(.systemGroupedBackground))` 删掉，整页 `.background(Theme.paper)`；
  - 文件头注释改成新结构的一两句。

- [ ] **Step 4：构建、跑单测**，预期全过。

- [ ] **Step 5：截图自查**（见 Task 10 的截图步骤，只跑 `ReviewShots/testAppPages` 与 `VisualWalkthrough/testWalkthrough`），对照 `cloud/design/mockups/02 记忆卡.dc.html#2c` 与 `05 记录与免费版.dc.html#2j`。模拟器默认没开通，看到的是 2c 那版；开通版靠 `UITests/SpaceShots.swift` 的流程（读它怎么进开通状态）。

- [ ] **Step 6：提交**

```bash
git add cloud/ios/App/Me cloud/ios/App/Account/AccountStore.swift
git commit -m "feat(cloud): 「我」照设计稿 2c / 2j：白页分组、懒得自己写、数据一组"
```

---

### Task 7：对象详情（1b）离开 List

**Files:** Modify `cloud/ios/App/Memory/ContactDetailView.swift`、`MaterialsSection.swift`、`MaterialRow.swift`

结构（`PaperPage(spacing: 14)`，设计稿 1b `.scroll` 的 gap 14）：

1. 失败横幅、待整理快满提示（同现在，放在 `PaperPage` 里最上面，左右 20）；
2. 头部：现在的 `header(contact)`，外面 `.padding(.horizontal, 20).padding(.top, 2).padding(.bottom, 4)`；
3. 每个种类：`PaperSectionTitle(text: kind.title)` + `PaperGroup`，组里每张卡一个按钮（点了 `editing = card`），卡之间 `PaperRowLine()`；有日子的卡用现在的 `.when` 行（`row(card)` 的第一支，外加 `.padding(.horizontal, 14).padding(.vertical, 12)`），没日子的用 `PaperMemRow(text: card.text)`；
4. 一张卡都没有：一组一行 `PaperRow(title: "还没有写下关于\(contact.name)的事")`，标题色 ink-3（给 `PaperRow` 传 `titleColor: Theme.ink3`）；
5. 「记一条」：一组一行，`Label("记一条", systemImage: "plus")`，`addCardButton` 色，按钮外加 `.padding(.horizontal, 14).padding(.vertical, 12)`；
6. `MaterialsSection`：见下；
7. DEBUG 的 `MemoryStressSection`：它是 `Section`，离开 `List` 后也得改成 `PaperSectionTitle("调试") + PaperGroup`，内容不变。

导航：
- 右上「设置」保持（`contactSettingsButton` = accentInk）；
- **返回键文字**：设计稿 1b 写「‹ 记得」、1d 写「‹ 小美」。给详情页设 `.navigationTitle(contact.name)`，再加 `ToolbarItem(placement: .principal) { EmptyView() }` 让顶栏不显示标题字；首页与通讯录的根视图各加 `.navigationTitle("记得")` / `.navigationTitle("通讯录")`（它们的导航栏是隐藏的，标题只用来给下一页当返回字）。截图核对返回键显示「记得」「通讯录」「小美」。

`MaterialsSection` 改成：

```swift
    var body: some View {
        if let error = store.loadError {
            PaperSectionTitle(text: MaterialDisplay.title(count: store.count))
            PaperGroup {
                MemoryFailureBanner(text: error) { Task { await store.reload(contactId) } }
                    .padding(.horizontal, 14)
                    .padding(.vertical, 12)
            }
        } else if let list = store.list, list.unprocessedCount > 0 {
            PaperSectionTitle(text: MaterialDisplay.title(count: list.unprocessedCount))
            PaperGroup {
                ForEach(Array(list.materials.enumerated()), id: \.element.id) { index, material in
                    if index > 0 { PaperRowLine() }
                    MaterialRow(
                        material: material,
                        expanded: expanded.contains(material.id),
                        deleting: store.deleting == material.id,
                        toggle: { toggle(material.id) },
                        delete: { confirming = material })
                }
                if !memoryReady {
                    PaperRowLine()
                    NavigationLink {
                        CloudIntroView()
                    } label: {
                        PaperRow(title: MaterialDisplay.cloudHint, chevron: true)
                    }
                    .buttonStyle(.plain)
                }
            }
            .alert(…)   // 原样搬过来
        }
    }
```

`MaterialRow`：去掉 `.swipeActions`（离开 `List` 不生效；展开后的「删除」还在），`.padding(.vertical, 2)` 换成 `.padding(.horizontal, 14).padding(.vertical, 12)`。文件头注释里「也可左滑」删掉；`UI 清单约束 7 第 17 条` 里「也可左滑」同步删掉（Task 9）。

- [ ] **Step 1：按上面改三个文件。** `ContactDetailView.swift` 若超过 200 行，把 `row(_:)` 搬进新文件 `App/Memory/ContactCardRow.swift`（`struct ContactCardRow: View { let card: MemoryCard … }`）。
- [ ] **Step 2：构建、跑单测**，预期全过。
- [ ] **Step 3：跑 UI 测试 `ReviewShots/testAppPages`、`ContactsShots`、`HomeFlow`**（Task 10 的命令）。它们若靠 `cells` / `staticTexts` 在 `List` 里找元素而失败，改成按 `accessibilityIdentifier` 或文字找，不要改产品行为去迁就测试。
- [ ] **Step 4：提交**

```bash
git add cloud/ios/App/Memory cloud/ios/App/Remember/RememberView.swift cloud/ios/App/Contacts/ContactsView.swift cloud/ios/UITests
git commit -m "feat(cloud): 对象详情照设计稿 1b 改成白页分组，返回键带上一页的名字"
```

---

### Task 8：改一条（1c）

**Files:** Modify `cloud/ios/App/Memory/CardEditor.swift`；Create `cloud/ios/App/Memory/KeywordChips.swift`

设计稿 1c 是盖在详情页上的底部弹层（`.sheet`：纸底、顶部圆角 22、`padding:10px 20px 40px`、`gap:14px`、顶上一根 36×5 的拖动条）。做法：

- 去掉 `NavigationStack` 与 `Form`，换成 `ScrollView { VStack(alignment: .leading, spacing: 14) { … } .padding(.horizontal, 20).padding(.top, 10).padding(.bottom, 40) }`，`.background(Theme.paper)`；
- 弹出方（`ContactDetailView` 的两个 `.sheet`）加 `.presentationDragIndicator(.visible)`、`.presentationCornerRadius(22)`；
- 顶部一行（`HStack`）：左「取消」（13 medium，`editorCancel`），中「改一条」/「记一条」（600 16），右「存好」（13 medium，`editorConfirm`，没字时 0.4 透明且不可点；保存中换 `MemorySavingLabel()`）；
- `PaperLabel("记忆")` + `TextField(..., axis: .vertical).lineLimit(2...5).modifier(PaperFieldStyle())`，下面右对齐计数（保留，12 ink-3）；
- `PaperLabel("是什么")` + 现在的 `KindPill` 胶囊排（不动，已经照 `.opts`）；
- 有日子时：`PaperLabel("到哪天")` + `PaperGroup { Button { 展开日历 } label: { PaperRow(title: MemoryDetailText.dayTitle(when), side: reminderNote) } }`，展开后在组下面放 `.graphical` 的 `DatePicker`（同现在）。`PaperGroup` 在弹层里不要左右 16 的外边距：给 `PaperGroup` 加一个 `inset: CGFloat = 16` 参数，这里传 0；
- `KeywordChips`（见下）；
- 编辑时最下面居中「删掉这条」（`.btn.danger`：13 medium ink），`.frame(maxWidth: .infinity).padding(.top, 8)`。

`KeywordChips.swift`：

```swift
// 改一条里的关键词（设计稿没画，UI 清单约束 7 第 14 条）：已有的词排成胶囊、各带 ×，下面一个输入框 +「添加」。
// 离开 Form 之后左滑删不了，改成胶囊上的 ×；上限与校验仍是 MemoryLimits。

import SwiftUI

struct KeywordChips: View {
    @Binding var keywords: [String]

    @State private var draft = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            PaperLabel(text: "关键词 \(keywords.count) / \(MemoryLimits.maxKeywords)")
            if !keywords.isEmpty {
                // iOS 17 没有流式布局容器，词少（最多 8 个、每个 ≤8 字）就横向滚
                ScrollView(.horizontal, showsIndicators: false) {
                    HStack(spacing: 8) {
                        ForEach(keywords, id: \.self) { word in
                            HStack(spacing: 4) {
                                Text(word).font(AppFont.font(size: 13))
                                Button {
                                    keywords.removeAll { $0 == word }
                                } label: {
                                    Image(systemName: "xmark").font(.system(size: 10, weight: .semibold))
                                }
                                .accessibilityLabel("删掉「\(word)」")
                            }
                            .foregroundStyle(Theme.ink2)
                            .padding(.horizontal, 12)
                            .frame(height: 32)
                            .overlay(Capsule().strokeBorder(Hairline.line))
                        }
                    }
                }
            }
            HStack(spacing: 8) {
                TextField("2–8 个字", text: $draft)
                    .modifier(PaperFieldStyle())
                Button("添加") {
                    keywords.append(draft.trimmingCharacters(in: .whitespacesAndNewlines))
                    draft = ""
                }
                .font(AppFont.font(size: 13, weight: .medium))
                .foregroundStyle(ColorUsage.editorSave.role.color)
                .disabled(!MemoryLimits.canAdd(draft, to: keywords))
            }
            Text("打字时出现这些词，键盘会提示这一条；不写就按这条的内容自动找。")
                .font(AppFont.font(size: 12))
                .foregroundStyle(Theme.ink3)
        }
    }
}
```

`CardEditor` 里原来的 `newKeyword` 状态删掉，关键词一节换成 `KeywordChips(keywords: $keywords)`。

- [ ] **Step 1：按上面改。** `CardEditor` 的 `load` / `save` / `close` 与 `.memoryEditorAlert` / `onChange(of: store.message == nil)` 原样保留。
- [ ] **Step 2：构建、跑单测**，预期全过。
- [ ] **Step 3：截图自查**：`ReviewShots/testAppPages` 里若没拍改一条，在 `ReviewShots.swift` 加一步：进详情点第一张卡、拍「card-editor」、点「取消」。对照 `02 记忆卡.dc.html#1c`。
- [ ] **Step 4：提交**

```bash
git add cloud/ios/App/Memory cloud/ios/App/Paper/PaperGroup.swift cloud/ios/UITests/ReviewShots.swift
git commit -m "feat(cloud): 改一条照设计稿 1c 改成底部弹层与自绘表单"
```

---

### Task 9：对象设置（1d）

**Files:** Modify `cloud/ios/App/Memory/ContactSettingsView.swift`

结构（`PaperPage`，缺省间距 18）：

1. 失败横幅（同现在）；
2. 组：「名字」行（右边 `TextField`，右对齐、ink、15）、`PaperRowLine()`、「在键盘上显示为」行（同现在的代号框与 12 字上限）——用 `PaperRow(title:) { TextField(...) }` 的 trailing 版本；
3. `PaperLead(text: "名字只保存在这台手机上。键盘上可以改用代号。", small: true)`，外加 `.padding(.top, -10)`（设计稿 `margin-top:-10px`）；
4. 组：「置顶」行（副标题 `MemoryStore.Wording.pinNote(store.pinnedCount)`，trailing 是 `Toggle("", isOn: pinBinding(contact)).labelsHidden().tint(appToggle)`）；
5. `PaperSectionTitle("提示里怎么称呼")` + `MemoryPronounPicker(selection: binding(contact, \.pronoun))`，左右 20（与加人页一致，替掉系统分段控件）；
6. 组：「打字时提示」开关行、`PaperRowLine()`、「日子提醒」开关行（副标题 `remindOffNote`）；组下 `PaperLead(text: 保存中 ? saving : switchesNote, small: true)`；
7. 有技能时：组一行「改写用哪个技能」，trailing 是 `Menu` 或 `Picker(...).pickerStyle(.menu)`（选项同现在），组下 `PaperLead` 说明（同现在的脚注）；
8. 组一行「导出记忆」›：`ShareLink(item: store.exportText(contactId)) { PaperRow(title: "导出记忆", chevron: true) }.buttonStyle(.plain)`；
9. 「忘掉这个人」：不在分组里，居中 13 medium ink（`.btn.danger`），`.frame(maxWidth: .infinity)`。

`.disabled(!store.canEdit || store.saving)` 包住 2–9。`sheet` / `onAppear` / `onDisappear` / 各 binding 与 `save` / `saveTexts` / `forgetIfChosen` 原样保留。

- [ ] **Step 1：按上面改。** 超过 200 行就把 2–9 的视图拆进 `App/Memory/ContactSettingsSections.swift`（`extension ContactSettingsView`，跨文件用到的私有方法标 `fileprivate` 不行，改 `internal` 或照 contributing「大类型的 impl 按职责拆」用 `pub(super)` 等价的 internal）。
- [ ] **Step 2：构建、跑单测**，预期全过。
- [ ] **Step 3：截图自查**，对照 `02 记忆卡.dc.html#1d`（设计稿里的「继续学习」「本机记忆包」没做，不画）。
- [ ] **Step 4：提交**

```bash
git add cloud/ios/App/Memory
git commit -m "feat(cloud): 对象设置照设计稿 1d 改成白页分组"
```

---

### Task 10：全量验证与截图

- [ ] **Step 1：单测全绿**（命令同上）。

- [ ] **Step 2：UI 截图走查**（专用模拟器 `$D`，第一次照 `cloud/ios/UITests/README.md` 准备：键盘排序、`EnableFullAccess`、`seed.py`）：

```bash
cd cloud/ios
xcodebuild build-for-testing -project QingjianCloud.xcodeproj -scheme QingjianCloud -destination "id=$D" -derivedDataPath /tmp/qjdd
xcrun simctl install $D /tmp/qjdd/Build/Products/Debug-iphonesimulator/QingjianCloud.app
python3 UITests/seed/seed.py --device $D
for t in ReviewShots/testAppPages VisualWalkthrough/testWalkthrough ContactsShots HomeFlow SpaceShots; do
  n=$(echo $t | tr '/' '-'); rm -rf /tmp/qjres-$n.xcresult /tmp/qjshots/$n; mkdir -p /tmp/qjshots/$n
  xcodebuild test-without-building -project QingjianCloud.xcodeproj -scheme QingjianCloud -destination "id=$D" \
    -derivedDataPath /tmp/qjdd -resultBundlePath /tmp/qjres-$n -only-testing:QingjianCloudUITests/$t 2>&1 | grep -E "\*\* TEST|error:"
  xcrun xcresulttool export attachments --path /tmp/qjres-$n.xcresult --output-path /tmp/qjshots/$n
done
```

预期每条 `** TEST EXECUTE SUCCEEDED **`。再切深色（`xcrun simctl ui $D appearance dark`）跑一遍 `ReviewShots/testAppPages` 导到 `/tmp/qjshots-dark/`，跑完改回 `light`。

- [ ] **Step 3：自查清单**（逐张看截图，不过的回对应任务改）：
  - 四页底色是纸色、不是系统分组灰；分组白底圆角 14、外圈很淡；行间线通栏；
  - 1b：返回键「‹ 记得」或「‹ 通讯录」，右上「设置」灰绿；种类小节标题小灰字；没日子的卡没有 meta 行；
  - 1c：底部弹层、拖动条、取消 / 标题 / 存好一行、字段是白框、「删掉这条」黑字居中；
  - 1d：返回键「‹ 小美」（或那个人的名字）；名字下小灰字；称呼是胶囊；「忘掉这个人」黑字居中；
  - 「我」没开通：最上面灰底「懒得自己写？」，数据一组只有「导出记忆卡」；开通版（SpaceShots）：素笺云服务一行、数据四行、后两行暗红；
  - 深色：字都看得清，没有白底黑字的块。

- [ ] **Step 4：文档**——`cloud/docs/plans/2026-10-05-ui-implementation.md`：
  - 逐屏表：02 的 1b / 1c / 1d 的「差」里删掉已照稿的部分（页面骨架、`.when`、`.opts`、`.btn.danger`）；05 的 2j 改「部分实现」并写清已有：懒得自己写（免费）、数据一组（交给谁处理、导出、撤回、删除素笺云服务）；02 的 2c 补一行（已实现）；
  - 约束 7 第 17 条删「也可左滑」；加一条：「删除账号」改为「删除素笺云服务」（文案待审计会话确认）；
  - 计数表同步。

```bash
git add cloud/docs/plans/2026-10-05-ui-implementation.md
git commit -m "docs(cloud): UI 清单同步四页改成设计稿白页之后的状态"
```

- [ ] **Step 5：推送并在 PR 里报告**

```bash
git push -u origin feat/ui-paper-pages
gh pr create --repo dayuer/qingjian --base fix/ui-align-design --head feat/ui-paper-pages \
  --title "feat(cloud): 对象详情、改一条、对象设置、我 照设计稿改成白页分组" --body "<做了什么 / 有意没照稿的 / 怎么验证的（系统版本、模拟器、截图目录）>"
```

（PR #9 合进 `sujian` 后把这个 PR 的 base 改成 `sujian`。）

- [ ] **Step 6：把截图目录（`/tmp/qjshots`、`/tmp/qjshots-dark`）、PR 链接、自查清单里没过的项、遇到的偏离写成一段，回给派活的会话。**
