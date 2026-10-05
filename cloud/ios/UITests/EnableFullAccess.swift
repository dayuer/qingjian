// 在模拟器里把素笺键盘的「允许完全访问」打开。
//
// 键盘的记忆类界面要这个开关才出得来（`KeyboardViewController` 读系统的 `hasFullAccess`，
// 扩展内无法伪造），而模拟器没有别的自动化办法，只能驱动系统设置走一遍：
// 设置 → 通用 → 键盘 → 键盘 → 素笺 → 允许完全访问 → 允许。
//
// 前提：素笺键盘已加进键盘列表（见 scripts 或 `simctl spawn defaults write .GlobalPreferences AppleKeyboards`）。
// 找不到元素时把当时的元素树打出来（attach），方便按系统版本调整。
import XCTest

final class EnableFullAccess: XCTestCase {
    private func dump(_ app: XCUIApplication, _ label: String) {
        let attachment = XCTAttachment(string: app.debugDescription)
        attachment.name = "tree-\(label)"
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    private func shot(_ name: String) {
        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = name
        attachment.lifetime = .keepAlways
        add(attachment)
    }

    /// 按标签点第一个可点的元素。设置页里同一标签会出现多次（行标题 + 小节标题 + 导航栏标题），
    /// 所以要遍历匹配项取第一个 hittable 的。
    /// 注意：`isHittable` **不能**写进 NSPredicate（会抛 XCTElementQueryInvalidPredicate），
    /// 只能在取到的元素上判。
    @discardableResult
    private func tapAny(_ app: XCUIApplication, _ labels: [String], timeout: TimeInterval = 5) -> Bool {
        for label in labels {
            let matches = app.descendants(matching: .any)
                .matching(NSPredicate(format: "label == %@", label))
            guard matches.firstMatch.waitForExistence(timeout: timeout) else { continue }
            for index in 0..<matches.count {
                let element = matches.element(boundBy: index)
                if element.isHittable {
                    element.tap()
                    return true
                }
            }
        }
        return false
    }

    /// 按 identifier 点。设置页里「键盘」这个标签同时是行标题和导航栏标题，按标签点会原地打转，
    /// 所以进键盘列表要认 identifier（`KEYBOARDS`）。
    @discardableResult
    private func tapIdentifier(_ app: XCUIApplication, _ identifier: String, timeout: TimeInterval = 5) -> Bool {
        let matches = app.descendants(matching: .any)
            .matching(NSPredicate(format: "identifier == %@", identifier))
        guard matches.firstMatch.waitForExistence(timeout: timeout) else { return false }
        for index in 0..<matches.count {
            let element = matches.element(boundBy: index)
            if element.isHittable {
                element.tap()
                return true
            }
        }
        return false
    }

    func testEnableFullAccess() throws {
        let settings = XCUIApplication(bundleIdentifier: "com.apple.Preferences")
        settings.launch()
        XCTAssertTrue(settings.wait(for: .runningForeground, timeout: 15))
        shot("settings-root")

        // 设置 → 通用
        guard tapAny(settings, ["通用", "General"]) else {
            dump(settings, "root"); return XCTFail("找不到「通用」")
        }
        Thread.sleep(forTimeInterval: 1)

        // 通用 → 键盘
        guard tapAny(settings, ["键盘", "Keyboard"]) else {
            dump(settings, "general"); return XCTFail("找不到「键盘」")
        }
        Thread.sleep(forTimeInterval: 1)

        // 键盘 → 键盘（子项，真正的键盘列表）。这一行的 identifier 是 KEYBOARDS，
        // 按标签点会撞上导航栏的「键盘」标题原地打转。
        guard tapIdentifier(settings, "KEYBOARDS") else {
            dump(settings, "keyboard"); return XCTFail("找不到键盘列表入口")
        }
        Thread.sleep(forTimeInterval: 1)
        shot("settings-keyboards")
        dump(settings, "keyboards")

        // 键盘列表 → 素笺。
        // 注意两点：
        //   1. `defaults write .GlobalPreferences AppleKeyboards` 在 iOS 26 模拟器上不足以注册第三方键盘
        //      （列表里不会出现素笺），得走「添加新键盘」这条正路，素笺在「第三方键盘」小节下。
        //   2. 在「添加新键盘」里点素笺只是**把它加进列表**并返回，不会打开详情页；
        //      要再回列表点一次素笺，才进得去有「允许完全访问」开关的那一页。
        if !tapAny(settings, ["素笺", "Sujian"], timeout: 3) {
            guard tapAny(settings, ["添加新键盘", "Add New Keyboard…"], timeout: 5) else {
                dump(settings, "keyboard-list"); return XCTFail("列表里既没有素笺，也没有「添加新键盘」")
            }
            Thread.sleep(forTimeInterval: 1)
            guard tapAny(settings, ["素笺", "Sujian"], timeout: 5) else {
                dump(settings, "add-keyboard"); return XCTFail("「添加新键盘」里没有素笺")
            }
            Thread.sleep(forTimeInterval: 1.5)
            guard tapAny(settings, ["素笺", "Sujian"], timeout: 5) else {
                dump(settings, "after-add"); return XCTFail("加进列表后点不到素笺")
            }
        }
        Thread.sleep(forTimeInterval: 1)
        shot("settings-sujian")
        dump(settings, "sujian-page")

        // 「允许完全访问」开关
        let toggle = settings.switches["允许完全访问"]
        let toggleEn = settings.switches["Allow Full Access"]
        let target = toggle.exists ? toggle : toggleEn
        guard target.waitForExistence(timeout: 5) else {
            dump(settings, "sujian-page"); return XCTFail("找不到「允许完全访问」开关")
        }
        if (target.value as? String) != "1" {
            // 直接 .tap() 打不开这个开关（点完 value 仍是 0），点它右侧的旋钮位置才生效
            target.coordinate(withNormalizedOffset: CGVector(dx: 0.92, dy: 0.5)).tap()
            Thread.sleep(forTimeInterval: 1.5)
            shot("settings-after-toggle")
            dump(settings, "after-toggle")
            // 系统弹的确认框（文案随系统版本变，逐个试）
            var confirmed = false
            for label in ["允许", "Allow", "好", "OK", "继续"] {
                let alertButton = settings.alerts.buttons[label]
                if alertButton.exists { alertButton.tap(); confirmed = true; break }
            }
            if !confirmed {
                // 弹窗可能不在 settings 进程里，试试 springboard
                let springboard = XCUIApplication(bundleIdentifier: "com.apple.springboard")
                for label in ["允许", "Allow", "好", "OK", "继续"] {
                    let alertButton = springboard.alerts.buttons[label]
                    if alertButton.exists { alertButton.tap(); confirmed = true; break }
                }
            }
            Thread.sleep(forTimeInterval: 1.5)
            XCTContext.runActivity(named: "确认框：\(confirmed ? "已点" : "没找到")") { _ in }
        }
        shot("settings-full-access-on")
        XCTAssertEqual(target.value as? String, "1", "开关没有打开")
    }
}
