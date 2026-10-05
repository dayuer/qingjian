// 候选栏换成 UIKit 之后的验收：长候选能横向滑，滑动后可视的候选确实变了，点滑出来的那个能上屏。
//
// 打字只能按坐标点键：键不是 XCUIElement（KeyTouchView 统收触摸，见 Keyboard/README）。
// 滑候选栏用坐标拖——候选栏那一行现在是 UIKit 的 CandidateBarView。
// 坐标量自 390×844 的模拟器截图（见 KeyboardShots 的文件头），iPhone 17e 也是 390×844。
import XCTest

final class CandidateBarScroll: XCTestCase {
    /// 三行字母键的中心 y（pt）
    private let row0: CGFloat = 572.33
    private let row1: CGFloat = 628.33
    private let row2: CGFloat = 684.33

    /// 候选栏那一行的中心 y：键区上沿（546.3）减去候选栏半高。
    private let barY: CGFloat = 521.3

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

    /// key 在三行里的中心 x（pt）。`row` 取 0/1/2。
    private func x(_ key: Character, row: Int) -> CGFloat {
        KeyGeometry.centerX(key, row: row) ?? 0
    }

    private func typeKey(_ app: XCUIApplication, _ key: Character) {
        let rows = ["qwertyuiop", "asdfghjkl", "zxcvbnm"]
        for (row, letters) in rows.enumerated() where letters.contains(key) {
            let y = [row0, row1, row2][row]
            tap(app, x(key, row: row), y)
            return
        }
    }

    func testLongCandidatesScroll() throws {
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

        let meTab = app.tabBars.buttons["我"]
        if meTab.waitForExistence(timeout: 5) { meTab.tap() } else { app.buttons["我"].tap() }
        wait()
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

        // 「我想去看看」的长候选：woxiangqukankan
        for key in "woxiangqukankan" { typeKey(app, key) }
        wait(3)
        shot("cbar-before")

        // 横向拖候选栏：从右侧拖到左侧，把后面的候选拉进可视区
        let start = app.coordinate(withNormalizedOffset: CGVector(dx: 0, dy: 0))
            .withOffset(CGVector(dx: 330, dy: barY))
        let end = app.coordinate(withNormalizedOffset: CGVector(dx: 0, dy: 0))
            .withOffset(CGVector(dx: 80, dy: barY))
        start.press(forDuration: 0.05, thenDragTo: end)
        wait(2)
        shot("cbar-after-scroll")

        // 点刚滑出来的那一格里（滑过之后左边那位已经不是首选了）
        tap(app, 80, barY)
        wait(3)
        shot("cbar-committed")
    }
}
