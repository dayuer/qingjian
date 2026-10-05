// 素笺 vs 系统「简体拼音」的输入手感对比：同一串拼音，两边各打一遍并逐屏截图，回来用脚本量。
//
// 全靠坐标驱动：键不是 XCUIElement（KeyTouchView 统收触摸），系统键盘那侧同样按坐标点。
// 坐标量自 390×844 的模拟器截图（见 KeyboardShots 的文件头）。
import XCTest

final class KeyboardCompare: XCTestCase {
    /// 三行字母键的中心 y（pt）
    private let rowY: [CGFloat] = [572.33, 628.33, 684.33]

    /// 候选栏那一行的中心 y：键区上沿（546.3）减去候选栏半高。
    private let barY: CGFloat = 521.3

    /// 键盘底部系统那一行（🌐 / 🎤）：从截图量出来是 (42.3, 805.2)。
    private let globeY: CGFloat = 805.2
    private let globeX: CGFloat = 42.3

    private let sample = "woxiangqukankan"

    override func setUp() {
        continueAfterFailure = true
    }

    private func shot(_ name: String) {
        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    private func dump(_ app: XCUIApplication, _ name: String) {
        let attachment = XCTAttachment(string: app.debugDescription)
        attachment.name = "tree-\(name)"
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

    /// 字母键在三行里的中心 x。
    private func x(_ key: Character) -> CGFloat {
        let rows = ["qwertyuiop", "asdfghjkl", "zxcvbnm"]
        let unit = (390 - 4 * 2 - 6.3 * 9) / 10
        let starts: [CGFloat] = [
            4,
            4 + unit * 1.36 + 6.3 + (280.92 - (unit * 9 + 6.3 * 8)) / 2,
            4 + unit * 1.36 + 6.3 + (280.92 - (unit * 7 + 6.3 * 6)) / 2,
        ]
        for (row, letters) in rows.enumerated() {
            let characters = Array(letters)
            guard let index = characters.firstIndex(of: key) else { continue }
            return starts[row] + CGFloat(index) * (unit + 6.3) + unit / 2
        }
        return 0
    }

    private func typeSample(_ app: XCUIApplication) {
        let rows = ["qwertyuiop", "asdfghjkl", "zxcvbnm"]
        for key in sample {
            for (row, letters) in rows.enumerated() where letters.contains(key) {
                tap(app, x(key), rowY[row])
            }
        }
    }

    /// 走到「我 → 键盘设置 → 试一试」，聚焦输入框。
    private func openField(_ app: XCUIApplication) {
        app.launch()
        wait(3)
        for label in ["开始", "先跳过", "先用免费版", "取消"] {
            let button = app.buttons[label]
            if button.waitForExistence(timeout: 3) { button.tap(); wait(1) }
        }
        let meTab = app.tabBars.buttons["我"]
        if meTab.waitForExistence(timeout: 5) { meTab.tap() } else { app.buttons["我"].tap() }
        wait()
        shot("me-page")
        dump(app, "me")
        for _ in 0..<10 {
            if app.buttons["键盘设置"].exists { break }
            app.swipeUp()
            wait(0.3)
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
        // 焦点落到输入框上就等键盘弹出来。不要用 app.keyboards 去判断——它在这套里不报键盘，
        // 也别反复点输入框（会把焦点点乱、页面也跟着跳）。看截图最实在。
        field.tap()
        wait(4)
        shot("field-focused")
    }

    /// 切到下一个键盘（点左下角地球键）。切换后键盘会重建，稍等。
    private func nextKeyboard(_ app: XCUIApplication) {
        tap(app, globeX, globeY)
        wait(3)
    }

    func testCompareSujianWithSystemPinyin() throws {
        let app = XCUIApplication()
        openField(app)

        // 依次点名各键盘，看看这台模拟器上是怎么排的（回来按图认哪个是素笺、哪个是系统拼音）
        shot("kb-0")
        for index in 1...5 {
            nextKeyboard(app)
            shot("kb-\(index)")
            // 有的键盘会让输入框失焦、键盘收起：再点回输入框，好接着切
            if !app.textViews["在这里试打"].exists && !app.textFields["在这里试打"].exists {
                let again = app.textViews.firstMatch.exists
                    ? app.textViews.firstMatch : app.textFields.firstMatch
                if again.exists { again.tap(); wait(2) }
            }
        }
    }

    /// 验收：长候选能横向滑、滑动后可视的候选确实变了、点滑出来的那个能上屏。
    /// 前后各截一张，回来用脚本比候选栏那一条的像素。
    func testCandidateBarScrolls() throws {
        let app = XCUIApplication()
        openField(app)
        typeSample(app)
        wait(3)
        shot("scroll-before")

        // 从候选栏右侧拖到左侧：把后面的候选拉进可视区
        let from = app.coordinate(withNormalizedOffset: CGVector(dx: 0, dy: 0))
            .withOffset(CGVector(dx: 330, dy: barY))
        let to = app.coordinate(withNormalizedOffset: CGVector(dx: 0, dy: 0))
            .withOffset(CGVector(dx: 60, dy: barY))
        from.press(forDuration: 0.05, thenDragTo: to)
        wait(2)
        shot("scroll-after")

        // 点滑过之后左边那一格：它不是首选了
        tap(app, 60, barY)
        wait(3)
        shot("scroll-committed")
    }
}
