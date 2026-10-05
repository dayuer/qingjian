// 通讯录（设计稿 02 的 2b）的截图走查：列表、展开一个人的记忆卡、搜索中。
// 截图走 XCUITest 的 attachment，跑完用 xcresulttool 导出；深色另跑一次（先 `simctl ui <设备> appearance dark`）。
// 数据由 /private/tmp/seed-contacts.py 种进 App Group，跑完删掉。

import XCTest

final class ContactsShots: XCTestCase {
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

    func testContacts() throws {
        let app = XCUIApplication()
        app.launch()
        wait(3)
        // 首次装可能出引导，四步都点掉
        for label in ["开始", "先跳过", "先用免费版", "取消"] {
            let button = app.buttons[label]
            if button.waitForExistence(timeout: 3) { button.tap(); wait(1) }
        }

        let contacts = app.tabBars.buttons["通讯录"]
        XCTAssertTrue(contacts.waitForExistence(timeout: 8), "找不到「通讯录」Tab")
        contacts.tap()
        wait(2)
        shot("02-2b-list")

        // 展开小美（种的第 6 个人）：下面要出她的几张记忆卡
        let 小美 = app.buttons["contact-00000000000000000000000000000006"]
        XCTAssertTrue(小美.waitForExistence(timeout: 5), "列表里没有小美")
        小美.tap()
        wait()
        shot("02-2b-expanded")

        // 展开另一个人：同时只展开一个，小美那条要收回去
        app.buttons["contact-00000000000000000000000000000001"].tap()
        wait()
        shot("02-2b-expanded-other")

        // 右上角「+」：先选建在哪个场景
        app.buttons["addContact"].tap()
        wait()
        shot("02-2b-add")
        app.staticTexts["通讯录"].firstMatch.tap()  // 收起菜单
        wait()

        // 搜索
        let field = app.textFields.firstMatch
        XCTAssertTrue(field.waitForExistence(timeout: 5), "没有搜索框")
        field.tap()
        app.typeText("小")
        wait()
        shot("02-2b-search")
    }
}
