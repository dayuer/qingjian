// 场景改成用户自建之后的截图走查：「我」页的场景一组、加一个、改名、删一个（人挪到默认场景）、通讯录里的场景名。
// 数据由 /private/tmp/seed-legacy.py 种成老样子（三个场景），跑起来先看迁移。

import XCTest

final class SceneShots: XCTestCase {
    override func setUp() {
        continueAfterFailure = true
    }

    private func shot(_ name: String) {
        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    private func wait(_ seconds: TimeInterval = 1.5) {
        Thread.sleep(forTimeInterval: seconds)
    }

    func testScenes() throws {
        let app = XCUIApplication()
        app.launch()
        wait(3)
        for label in ["开始", "先跳过", "先用免费版", "取消"] {
            let button = app.buttons[label]
            if button.waitForExistence(timeout: 3) { button.tap(); wait(1) }
        }

        app.tabBars.buttons["通讯录"].tap()
        wait(2)
        shot("scene-01-contacts")

        app.tabBars.buttons["我"].tap()
        wait(2)
        shot("scene-02-me")

        // 加一个场景
        let add = app.buttons["加一个场景"]
        XCTAssertTrue(add.waitForExistence(timeout: 5), "「我」页里没有「加一个场景」")
        XCTAssertTrue(add.isEnabled, "「加一个场景」是灰的")
        XCTAssertTrue(add.isHittable, "「加一个场景」点不到")
        add.tap()
        wait(2)
        shot("scene-03a-after-tap")
        let field = app.textFields.firstMatch
        XCTAssertTrue(field.waitForExistence(timeout: 10), "没有场景名输入框")
        field.tap()
        app.typeText("家人")
        wait()
        shot("scene-03-add")
        app.buttons["存好"].tap()
        wait(2)
        shot("scene-04-added")

        // 点进新场景：改名与删掉（行的标签是两行文字，按文字点）
        app.staticTexts["家人"].firstMatch.tap()
        wait()
        shot("scene-05-settings")
        app.buttons["改名"].tap()
        wait()
        let rename = app.textFields.firstMatch
        if rename.waitForExistence(timeout: 5) {
            rename.tap()
            let current = rename.value as? String ?? ""
            for _ in 0..<current.count { rename.typeText(XCUIKeyboardKey.delete.rawValue) }
            app.typeText("自己人")
            app.buttons["存好"].tap()
        }
        wait(2)
        shot("scene-06-renamed")

        // 删掉这个场景：里面没人，直接删
        app.buttons["删掉这个场景"].tap()
        wait()
        shot("scene-07-confirm")
        if app.buttons["删掉"].waitForExistence(timeout: 3) { app.buttons["删掉"].tap() }
        wait(2)
        shot("scene-08-deleted")
    }
}
