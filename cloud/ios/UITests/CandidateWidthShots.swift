// 候选格宽度的对稿截图：同几组输入的候选栏、滑到后面的候选栏、展开面板，改格子排版前后各跑一遍逐像素比对。
// 输入覆盖 emoji（haha）、英文与长英文词（hello）、长整句（woxiangqukankan）。
//
// 前提与坐标同 CandidateBarScroll：素笺键盘已加、完全访问已开，390×844 的模拟器。
import XCTest

final class CandidateWidthShots: XCTestCase {
    /// 三行字母键的中心 y（pt）
    private let rows: [CGFloat] = [572.33, 628.33, 684.33]

    /// 候选栏那一行的中心 y
    private let barY: CGFloat = 521.3

    /// ⌄ 的中心 x：屏宽 390 减去 ⌄ 半宽 24
    private let chevronX: CGFloat = 366

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

    private func tap(_ app: XCUIApplication, _ x: CGFloat, _ y: CGFloat) {
        app.coordinate(withNormalizedOffset: CGVector(dx: 0, dy: 0))
            .withOffset(CGVector(dx: x, dy: y))
            .tap()
        wait(0.2)
    }

    private func type(_ app: XCUIApplication, _ text: String) {
        let letters = ["qwertyuiop", "asdfghjkl", "zxcvbnm"]
        for key in text {
            for (row, line) in letters.enumerated() where line.contains(key) {
                tap(app, KeyGeometry.centerX(key, row: row) ?? 0, rows[row])
            }
        }
    }

    /// 进 App →「我」→ 键盘设置 →「试一试」输入框，弹出素笺键盘。
    private func openField() -> XCUIApplication {
        let app = XCUIApplication()
        app.launch()
        wait(3)
        for label in ["开始", "先跳过", "先用免费版", "取消"] {
            let button = app.buttons[label]
            if button.waitForExistence(timeout: 3) {
                button.tap()
                wait(1)
            }
        }
        // App 会恢复上次停在的页面（比如某人的详情），先一路返回到有标签栏的那一层
        for _ in 0..<5 where app.navigationBars.buttons["BackButton"].exists && !app.tabBars.buttons["我"].isHittable {
            app.navigationBars.buttons["BackButton"].tap()
            wait(0.6)
        }
        let meTab = app.tabBars.buttons["我"]
        if meTab.waitForExistence(timeout: 5) { meTab.tap() } else { app.buttons["我"].tap() }
        wait()
        for _ in 0..<3 where app.navigationBars.buttons["BackButton"].exists {
            app.navigationBars.buttons["BackButton"].tap()
            wait(0.6)
        }
        app.buttons["键盘设置"].tap()
        wait()
        for _ in 0..<8 {
            if app.textViews["在这里试打"].exists || app.textFields["在这里试打"].exists { break }
            app.swipeUp()
            wait(0.4)
        }
        let field = app.textViews["在这里试打"].exists
            ? app.textViews["在这里试打"] : app.textFields["在这里试打"]
        XCTAssertTrue(field.waitForExistence(timeout: 5), "没有「试一试」输入框")
        field.tap()
        wait(3)
        return app
    }

    func testCandidateWidths() throws {
        for input in ["haha", "hello", "woxiangqukankan"] {
            let app = openField()
            type(app, input)
            wait(3)
            shot("cw-\(input)-bar")

            // 往左拖三次，把排在后面的候选拉进可视区
            for _ in 0..<3 {
                let start = app.coordinate(withNormalizedOffset: CGVector(dx: 0, dy: 0))
                    .withOffset(CGVector(dx: 330, dy: barY))
                let end = app.coordinate(withNormalizedOffset: CGVector(dx: 0, dy: 0))
                    .withOffset(CGVector(dx: 20, dy: barY))
                start.press(forDuration: 0.05, thenDragTo: end)
                wait(1)
            }
            wait(1)
            shot("cw-\(input)-bar-scrolled")

            tap(app, chevronX, barY)
            wait(2)
            shot("cw-\(input)-panel")
            app.terminate()
        }
    }
}
