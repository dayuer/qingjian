// 键盘泄漏排查：首页「+ 记一条」弹出编辑框（键盘出现）→ 停 3 秒 → 取消（键盘收起），来回 5 次。
// 外面用 `log stream` 收 `sujian.keyboard` 的 live= 行，看每次收起后控制器、键盘模型、引擎的存活个数会不会归零。
import XCTest

final class KeyboardLeakCycle: XCTestCase {
    private func tap(_ app: XCUIApplication, _ x: CGFloat, _ y: CGFloat) {
        app.coordinate(withNormalizedOffset: CGVector(dx: 0, dy: 0))
            .withOffset(CGVector(dx: x, dy: y)).tap()
    }

    func testShowAndHideKeyboardFiveTimes() throws {
        let app = XCUIApplication()
        app.launch()
        Thread.sleep(forTimeInterval: 3)
        for label in ["开始", "先跳过", "先用免费版"] {
            let button = app.buttons[label]
            if button.waitForExistence(timeout: 2) { button.tap() }
        }
        for _ in 0..<5 {
            // 「+ 记一条」按标签找不到时按坐标点（同 KeyboardMemorySoak）
            let compose = app.buttons["+ 记一条"]
            if compose.waitForExistence(timeout: 3) { compose.tap() } else { tap(app, 332, 81) }
            Thread.sleep(forTimeInterval: 3)
            tap(app, 195, 250)
            Thread.sleep(forTimeInterval: 3)
            let cancel = app.buttons["取消"]
            if cancel.waitForExistence(timeout: 3) { cancel.tap() } else { tap(app, 49, 85) }
            Thread.sleep(forTimeInterval: 3)
        }
    }
}
