// 截图走查：把 App 驱动到设计稿的每一屏并截图，交 UI 审计员对稿
// （见 cloud/docs/plans/2026-10-05-ui-implementation.md 的「约束」与 T1–T13）。
//
// 截图走 XCUITest 的 attachment，跑完用 xcresulttool export attachments 导出。
// 设计稿的模拟器是 390×844 @3x = 1170×2532，与 iPhone 14 的真机截图同尺寸，导出后可直接叠图。
//
// 注意：键盘的键**不是** XCUIElement —— `KeyTouchView` 是盖在键区上的 UIKit 视图统收触摸
// （见 Keyboard/README「键区的触摸」）。要点键得用归一化坐标，别用 app.buttons[...]。
import XCTest

final class VisualWalkthrough: XCTestCase {
    override func setUp() {
        continueAfterFailure = true
    }

    /// 截当前整屏并按屏号命名（如 01-1a）。
    private func shot(_ name: String) {
        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    private func wait(_ seconds: TimeInterval = 1.0) {
        Thread.sleep(forTimeInterval: seconds)
    }

    func testWalkthrough() throws {
        let app = XCUIApplication()
        app.launch()
        wait(3)

        // ── 首次引导（04 的 1a；05 的 2d、2e、2g）──
        stepOnboarding(app)

        // ── 三个 Tab ──
        tab(app, "记得", shotAs: "02-2a-tab")
        tab(app, "通讯录", shotAs: "02-2b-tab")
        tab(app, "我", shotAs: "05-2j")
    }

    /// 引导四步：介绍 → 开启键盘 → 免费版与云服务 → 第一个对象。
    private func stepOnboarding(_ app: XCUIApplication) {
        // 第一步「介绍」：底部「开始」
        let start = app.buttons["开始"]
        guard start.waitForExistence(timeout: 8) else {
            return  // 已经看过引导，不再出
        }
        shot("04-1a")
        start.tap()
        wait()

        // 第二步「开启键盘」（05 的 2d）：有「先跳过」次按钮
        shot("05-2d")
        if app.buttons["先跳过"].exists {
            app.buttons["先跳过"].tap()
        } else if app.buttons["下一步"].exists {
            app.buttons["下一步"].tap()
        }
        wait()

        // 第三步「免费版与云服务」（05 的 2e）
        shot("05-2e")
        if app.buttons["先用免费版"].exists {
            app.buttons["先用免费版"].tap()
        } else if app.buttons["下一步"].exists {
            app.buttons["下一步"].tap()
        }
        wait()

        // 第四步「第一个对象」（05 的 2g）
        shot("05-2g")
        if app.buttons["取消"].exists {
            app.buttons["取消"].tap()
        }
        wait()
    }

    private func tab(_ app: XCUIApplication, _ title: String, shotAs name: String) {
        let button = app.tabBars.buttons[title]
        if button.exists {
            button.tap()
        } else {
            app.buttons[title].tap()
        }
        wait()
        shot(name)
    }

    /// 「记得」的标题、状态行与日历条要钉住不滚（设计稿里它们在滚动区外面）。
    /// 往上滑一屏，前后各截一张，回来量标题的字顶有没有动。
    func testRememberHeaderStaysFixed() throws {
        let app = XCUIApplication()
        app.launch()
        wait(3)
        for label in ["开始", "先跳过", "先用免费版", "取消"] {
            let button = app.buttons[label]
            if button.waitForExistence(timeout: 3) { button.tap(); wait(1) }
        }
        let remember = app.tabBars.buttons["记得"]
        if remember.waitForExistence(timeout: 5) { remember.tap() }
        wait(2)
        shot("fixed-before")
        app.swipeUp()
        wait(2)
        shot("fixed-after")
    }
}
