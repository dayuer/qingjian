// 键盘部分的截图：进 App →「我」→ 键盘设置 →「试一试」输入框，素笺键盘弹出后**按坐标点键**组字。
//
// 为什么不用 `typeText`：键**不是** XCUIElement —— `KeyTouchView` 是盖在键区上的 UIKit 视图统收触摸
// （见 Keyboard/README「键区的触摸」），`typeText` 合成的是文本输入事件，绕过键盘自己的引擎，
// 结果是字母原样落进输入框、不组字、也不出候选与提示行。
//
// 坐标来源：从 `kbd-empty.png` 量出的键行（键高 134px、行距 168px，与 KeyStyle 的 44.5/11.5pt 一致），
// 再按 unit = (390 - 4*2 - 6.3*9)/10 = 32.53pt 算每个键的中心。键盘**底部锚定**，打字前后四行键的位置不变
// （已比对 kbd-empty 与 kbd-kaoshi），所以这些坐标可以写死。设备像素 ÷3 得 pt。
//
// 前提：素笺键盘已加进键盘列表、且「允许完全访问」已开（见 EnableFullAccess），
// 且 App Group 的 memory/ 里有对象与卡片（否则提示行没有内容可出）：`UITests/seed/seed.py` 种的小美有「考试」这张卡。
import XCTest

final class KeyboardShots: XCTestCase {
    /// qwerty 行中心 y（pt）
    private let row0: CGFloat = 572.33

    /// asdf / zxcv 行中心 y（pt）
    private let row1: CGFloat = 628.33

    override func setUp() {
        continueAfterFailure = true
    }

    private func shot(_ name: String) {
        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    private func wait(_ seconds: TimeInterval = 1.0) {
        Thread.sleep(forTimeInterval: seconds)
    }

    /// 按 pt 坐标点一下屏幕（相对 App 窗口左上角）。键盘由扩展进程画在 App 之上，
    /// 但命中测试走最上层窗口，所以从 App 的坐标原点起算仍然点得到。
    private func tapKey(_ app: XCUIApplication, _ x: CGFloat, _ y: CGFloat) {
        app.coordinate(withNormalizedOffset: CGVector(dx: 0, dy: 0))
            .withOffset(CGVector(dx: x, dy: y))
            .tap()
        wait(0.25)
    }

    /// 走到「我 → 键盘设置 → 试一试」并聚焦输入框，返回该输入框元素。
    private func openTryTypingField(_ app: XCUIApplication) -> XCUIElement? {
        app.launch()
        wait(3)

        // 首次引导会挡在前面，先走完/跳过
        for label in ["开始", "先跳过", "先用免费版", "取消"] {
            let button = app.buttons[label]
            if button.waitForExistence(timeout: 3) {
                button.tap()
                wait(1)
            }
        }

        let meTab = app.tabBars.buttons["我"]
        if meTab.waitForExistence(timeout: 5) { meTab.tap() } else { app.buttons["我"].tap() }
        wait()

        let settingsLink = app.buttons["键盘设置"]
        guard settingsLink.waitForExistence(timeout: 5) else {
            XCTFail("「我」页没有「键盘设置」")
            return nil
        }
        settingsLink.tap()
        wait()

        // 「试一试」在页面最下方，要先滚下去
        for _ in 0..<8 {
            if app.textViews["在这里试打"].exists || app.textFields["在这里试打"].exists { break }
            app.swipeUp()
            wait(0.4)
        }

        // `TextField(..., axis: .vertical)` 在 SwiftUI 里渲染成多行 textView，不是 textField，两种都试。
        let asTextView = app.textViews["在这里试打"]
        let asTextField = app.textFields["在这里试打"]
        let field = asTextView.waitForExistence(timeout: 3) ? asTextView : asTextField
        guard field.exists else {
            XCTFail("键盘设置里没有「试一试」输入框")
            return nil
        }
        return field
    }

    /// 空键盘的样子（没有组字、没有提示行）。
    func testKeyboardIdle() throws {
        let app = XCUIApplication()
        guard let field = openTryTypingField(app) else { return }
        field.tap()
        wait(3)
        shot("01-idle")

        // 工具栏左端的牌子（圆点 + 人名，量自 01-idle.png：bbox 中心 38.3pt, 521.3pt）→ 工具栏里横列其他人 +「不指定」
        tapKey(app, 38.3, 521.3)
        wait(2.5)
        shot("01-quick-picks")
    }

    /// 点键打 `kaoshi`：应出候选栏与记忆提示行（提示行的文案来自 App Group 里的卡片）。
    func testKeyboardTyping() throws {
        let app = XCUIApplication()
        guard let field = openTryTypingField(app) else { return }
        field.tap()
        wait(3)

        // k a o s h i —— 坐标为键中心（pt）
        tapKey(app, 311.33, row1)   // k
        tapKey(app, 39.33, row1)    // a
        tapKey(app, 330.67, row0)   // o
        tapKey(app, 78.33, row1)    // s
        tapKey(app, 233.67, row1)   // h
        tapKey(app, 291.67, row0)   // i
        wait(3)
        shot("01-1a-typing")

        // 提示行右端的展开箭头（量自 01-1a-typing.png：最右深色簇中心 370.0pt，提示行中线 478.7pt）
        // → 对象卡（01 的 1b）
        tapKey(app, 370.0, 478.7)
        wait(2.5)
        shot("01-1b")
    }
}
