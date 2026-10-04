// 底部三栏：顺序、标题与图标名照设计稿 02 第 2 轮「记得 · 通讯录 · 我」，图标在资源目录里找得到。

import UIKit
import XCTest
@testable import QingjianCloud

final class RootTabTests: XCTestCase {
    func testOrder() {
        XCTAssertEqual(RootTab.allCases, [.remember, .contacts, .me])
    }

    func testTitles() {
        XCTAssertEqual(RootTab.allCases.map(\.title), ["记得", "通讯录", "我"])
    }

    func testIcons() {
        XCTAssertEqual(RootTab.allCases.map(\.icon), ["tab-calendar", "tab-contacts", "tab-me"])
    }

    func testIconsAreTemplateImagesInAppBundle() {
        for tab in RootTab.allCases {
            let image = UIImage(named: tab.icon, in: Bundle(for: MemoryStore.self), compatibleWith: nil)
            XCTAssertNotNil(image, tab.icon)
            XCTAssertEqual(image?.renderingMode, .alwaysTemplate, tab.icon)
        }
    }
}
