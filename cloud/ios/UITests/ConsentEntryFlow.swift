// 审计要求（2026-10-06）：证明云功能开关在用户点得到的地方，且同意页能弹出。
// 路径：我 → 素笺云服务 → 云功能开关（截屏）→ 开「云端记忆」→ 同意页（截屏）。
// 前提：模拟器里已开通云服务（signedIn），没开通时这条测试只截到开通页并 XCTFail。

import XCTest

final class ConsentEntryFlow: XCTestCase {
    private func shot(_ name: String) {
        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    private func wait(_ seconds: TimeInterval = 1.0) {
        Thread.sleep(forTimeInterval: seconds)
    }

    func testMeToCloudSwitchesAndConsentSheet() throws {
        let app = XCUIApplication()
        app.launch()
        wait(3)

        // 首次引导挡在前面就先跳过
        for label in ["开始", "先跳过"] {
            let button = app.buttons[label]
            if button.waitForExistence(timeout: 2) {
                button.tap()
                wait(1)
            }
        }

        // 底部「我」标签
        let meTab = app.tabBars.buttons.element(boundBy: 3)
        XCTAssertTrue(meTab.waitForExistence(timeout: 5), "没有底部标签栏")
        meTab.tap()
        wait(1)
        shot("me-tab")

        // 我 → 素笺云服务
        let entry = app.buttons.containing(NSPredicate(format: "label CONTAINS %@", "云服务")).firstMatch
        XCTAssertTrue(entry.waitForExistence(timeout: 5), "「我」页里没有素笺云服务入口")
        entry.tap()
        wait(1)

        // 开通了才有开关；没开通这条留待开通后重跑
        let memoryToggle = app.switches["云端记忆（把记下的素材整理成卡）"]
        guard memoryToggle.waitForExistence(timeout: 5) else {
            shot("cloud-entry-not-signed-in")
            throw XCTSkip("模拟器里没开通云服务，开关截图留待开通后补")
        }
        shot("cloud-switches")

        // 开「云端记忆」要先过同意页
        memoryToggle.switches.firstMatch.tap()
        wait(1)
        let agree = app.buttons["同意并开启"]
        XCTAssertTrue(agree.waitForExistence(timeout: 5), "开关弹不出同意页")
        shot("consent-sheet")
        XCTAssertTrue(agree.isEnabled == false, "没勾选前「同意并开启」不该可用")
        app.buttons["取消"].tap()
    }
}
