// 开通云服务这条路要给 UI 审计员对的四屏：我页那一行、开通页（未勾 / 已勾）、输码页。
// 设计稿没有这几屏（UI 清单 D6），照系统表单做，这几张是给审计员看实际长什么样的。
// **不按「开通」**：随包的 cloud.toml 指向的是线上地址（`pinyin.synon.ai`），按下去会真的发一次建空间请求。
// 成功与失败态要另起一个假服务端、把 cloud.toml 指过去才有（见 UITests/README.md）。
// 深色另跑一次（先 `xcrun simctl ui <设备> appearance dark`）。

import UIKit
import XCTest

final class SpaceShots: XCTestCase {
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

    func testTheOpenSpaceScreens() throws {
        let app = launch()

        let me = app.tabBars.buttons["我"]
        XCTAssertTrue(me.waitForExistence(timeout: 5), "找不到「我」Tab")
        me.tap()
        wait(1.5)
        shot("我页 · 素笺云服务那一行")

        // SwiftUI 的 NavigationLink 会把标题与右侧的值合成一个可访问元素，标签是「素笺云服务, 没开通」
        let row = app.descendants(matching: .any)
            .matching(NSPredicate(format: "label BEGINSWITH %@", "素笺云服务"))
            .firstMatch
        XCTAssertTrue(row.waitForExistence(timeout: 5), "找不到「素笺云服务」那一行")
        row.tap()
        wait(1.5)
        shot("开通页 · 未勾同意")

        // 同意那一行的勾选框：设计稿 .check，这里是一个 Toggle
        let consent = app.switches.firstMatch
        XCTAssertTrue(consent.waitForExistence(timeout: 5), "找不到出境同意的勾选框")
        consent.tap()
        wait(0.6)
        shot("开通页 · 已勾同意")

        // 「开通」按钮这时该是可点的（勾了同意）
        XCTAssertTrue(app.buttons["开通"].firstMatch.isEnabled, "勾了同意之后「开通」该能点")

        let join = app.buttons["已经有素笺云服务了？用匹配码加入"].firstMatch
        XCTAssertTrue(join.waitForExistence(timeout: 5), "找不到输码入口")
        join.tap()
        wait(1.5)
        shot("输码页 · 空")

        let field = app.textFields["匹配码"].firstMatch
        XCTAssertTrue(field.waitForExistence(timeout: 5), "找不到匹配码输入框")
        field.tap()
        field.typeText("K7P2-9QXM")
        wait(0.6)
        shot("输码页 · 输满了")
    }
}
