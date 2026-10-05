// 「先快速记、之后再整理」的端到端：首页 →「+ 记一条」写一句 → 当天事件里出现（未归人）→
// 点「补上」选个人 → 卡归过去。
//
// 前提：App Group 的 memory/ 里至少有一个对象（选人那步要有人可选）。
import XCTest

final class HomeFlow: XCTestCase {
    private func shot(_ name: String) {
        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    private func wait(_ seconds: TimeInterval = 1.0) {
        Thread.sleep(forTimeInterval: seconds)
    }

    func testQuickNoteThenAssign() throws {
        let app = XCUIApplication()
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

        // 「+ 记一条」写一句（按 accessibilityIdentifier 找，不靠 label 拼法）
        let compose = app.buttons["quickNote"]
        XCTAssertTrue(compose.waitForExistence(timeout: 5), "首页没有「+ 记一条」")
        compose.tap()
        wait(1)

        let editor = app.textViews.firstMatch
        XCTAssertTrue(editor.waitForExistence(timeout: 5), "快速记的输入框没出来")
        editor.tap()
        editor.typeText("她提过想去看海")
        wait(0.5)
        shot("quicknote-typed")

        app.buttons["记下"].tap()
        wait(2)
        shot("quicknote-saved")

        // 当天事件里应出现这张卡，带「补上」。未归人的卡可能不止一张（种子里还有一张），取第一个。
        let assign = app.buttons["补上"].firstMatch
        XCTAssertTrue(assign.waitForExistence(timeout: 5), "首页没出现「补上」")
        assign.tap()
        wait(1.5)
        shot("assign-sheet")

        // 在弹层里点第一个人。**别用 `collectionViews.firstMatch.cells.firstMatch`**：
        // 那个匹配到的可能是弹层后面的视图，点下去落在遮罩上，弹层被关掉、`assign` 根本没被调用。
        let person = app.buttons["小美"]
        XCTAssertTrue(person.waitForExistence(timeout: 5), "选人弹层里没有「小美」")
        person.tap()
        wait(5)
        shot("assigned")
        // 归人失败时弹层不会自己收（assign 返回 false），把那一刻的元素树打出来看原因
        let tree = XCTAttachment(string: app.debugDescription)
        tree.name = "after-assign"
        tree.lifetime = .keepAlways
        add(tree)
    }
}
