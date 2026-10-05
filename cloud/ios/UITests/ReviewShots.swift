// PR #4 修完后给审计对稿用的截图：记一笔的确认条（1e）、草稿卡（1e-2，没有提示行）、冲突屏（1e-3，原话在、人名对），
// 「我」页顶部、场景删除确认（alert，「算了」要在）、三页与对象详情的列表末尾（让出浮动 Tab 栏），以及坏卡在时场景改名照样成功。
// 数据：`UITests/seed/seed.py`（坏卡那条用 `--bad-card`）；键盘那条另要素笺键盘已加、完全访问已开（见 UITests/README.md）。
// 深色另跑一次（先 `xcrun simctl ui <设备> appearance dark`）。

import UIKit
import XCTest

final class ReviewShots: XCTestCase {
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
        wait(0.4)
    }

    private func launch() -> XCUIApplication {
        let app = XCUIApplication()
        app.launch()
        wait(3)
        for label in ["开始", "先跳过", "先用免费版", "取消"] {
            let button = app.buttons[label]
            if button.waitForExistence(timeout: 2) { button.tap(); wait(1) }
        }
        return app
    }

    private func tab(_ app: XCUIApplication, _ title: String) {
        let button = app.tabBars.buttons[title]
        XCTAssertTrue(button.waitForExistence(timeout: 5), "找不到「\(title)」Tab")
        button.tap()
        wait(1.5)
    }

    private func scrollToEnd(_ app: XCUIApplication) {
        for _ in 0..<5 {
            app.swipeUp()
            wait(0.4)
        }
        wait(1)
    }

    /// 「我」页顶部、场景删除确认、四个列表的末尾。
    func testAppPages() throws {
        let app = launch()

        tab(app, "我")
        shot("me-top")
        scrollToEnd(app)
        shot("me-end")

        tab(app, "记得")
        scrollToEnd(app)
        shot("remember-end")

        tab(app, "通讯录")
        scrollToEnd(app)
        shot("contacts-end")

        // 对象详情：小美的卡最多
        app.swipeDown()
        app.swipeDown()
        wait()
        let person = app.buttons["contact-00000000000000000000000000000006"]
        if person.waitForExistence(timeout: 5) {
            person.tap()
            wait()
            app.buttons.matching(NSPredicate(format: "label BEGINSWITH '全部'")).firstMatch.tap()
            wait(1.5)
            scrollToEnd(app)
            shot("detail-end")
        }

        // 场景删除确认：iOS 26 上要是 alert，「算了」得在
        tab(app, "我")
        app.tabBars.buttons["我"].tap()
        wait()
        app.staticTexts["工作"].firstMatch.tap()
        wait(1.5)
        app.buttons["删掉这个场景"].tap()
        wait(1.5)
        shot("scene-delete-alert")
        XCTAssertTrue(app.alerts.firstMatch.waitForExistence(timeout: 3), "删场景的确认不是 alert")
        XCTAssertTrue(app.alerts.buttons["算了"].exists, "确认框里没有「算了」")
        XCTAssertTrue(app.alerts.buttons["删掉"].exists, "确认框里没有「删掉」")
        app.alerts.buttons["算了"].tap()
        wait()
    }

    /// 种了坏卡（`seed.py --bad-card`）时给「日常」改名：照样成功，不弹「每个关键词要 2 到 8 个字」。
    func testRenameSceneWithBadCard() throws {
        let app = launch()
        tab(app, "我")
        app.staticTexts["日常"].firstMatch.tap()
        wait(1.5)
        app.buttons["改名"].tap()
        wait()
        let field = app.textFields.firstMatch
        XCTAssertTrue(field.waitForExistence(timeout: 5), "没有场景名输入框")
        field.tap()
        let current = field.value as? String ?? ""
        for _ in 0..<current.count { field.typeText(XCUIKeyboardKey.delete.rawValue) }
        app.typeText("家里")
        app.buttons["存好"].tap()
        wait(2)
        shot("bad-card-rename")
        XCTAssertFalse(app.alerts.firstMatch.exists, "改名弹了错误提示")
        let renamed = app.staticTexts.matching(NSPredicate(format: "label CONTAINS '家里'")).firstMatch
        XCTAssertTrue(renamed.exists, "改名没成")
        app.navigationBars.buttons.firstMatch.tap()
        wait(1.5)
        shot("bad-card-rename-list")
    }

    /// 记一笔：确认条（1e）→ 草稿卡（1e-2）→ 冲突屏（1e-3）。键盘的坐标量自 390×844 的截图（键盘底部锚定）。
    func testNoteFlow() throws {
        // 键盘记着处理过的剪贴板（同一段不再出确认条），每次跑换一句：计数存在测试进程自己的 UserDefaults 里
        let samples = [
            "下个月想去厦门，听说沙坡尾很好逛", "下周六想去爬山，她说想看日出", "周末想去看展，她最近喜欢印象派",
            "月底想去杭州，听说西溪很安静", "明年春天想去京都看樱花",
        ]
        let run = UserDefaults.standard.integer(forKey: "reviewShotsNoteRun")
        UserDefaults.standard.set(run + 1, forKey: "reviewShotsNoteRun")
        UIPasteboard.general.string = samples[run % samples.count]
        let app = launch()
        tab(app, "我")
        app.buttons["键盘设置"].tap()
        wait()
        for _ in 0..<8 where !app.textViews["在这里试打"].exists && !app.textFields["在这里试打"].exists {
            app.swipeUp()
            wait(0.4)
        }
        let field = app.textViews["在这里试打"].exists ? app.textViews["在这里试打"] : app.textFields["在这里试打"]
        field.tap()
        wait(3)
        shot("note-0-idle")

        // 工具栏的「记一笔」
        tap(app, 318, 521.3)
        wait(1.5)
        let springboard = XCUIApplication(bundleIdentifier: "com.apple.springboard")
        for label in ["允许粘贴", "Allow Paste"] where springboard.buttons[label].exists {
            springboard.buttons[label].tap()
            wait(1.5)
        }
        shot("note-1e-clip")

        // 确认条右边的「记到小美」
        tap(app, 340, 472)
        wait(1.5)
        shot("note-1e2-draft")

        // 草稿卡底部的「记下 n 条」
        tap(app, 250, 748)
        wait(1.5)
        shot("note-1e3-conflict")
    }
}
