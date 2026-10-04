// 没开完全访问那一屏与对象无关：颜色全是中性色，不引用 accent；对象相关的面板仍用强调色。

import XCTest
@testable import QingjianCloud

final class NoFullAccessStyleTests: XCTestCase {
    func testNoFullAccessScreenHasNoAccent() {
        XCTAssertFalse(NoFullAccessStyle.standard.roles.contains(.accent))
    }

    func testButtonIsOutlinedWithNeutralColors() {
        let style = NoFullAccessStyle.standard
        XCTAssertEqual(style.closeText, .ink)
        XCTAssertEqual(style.buttonText, .ink)
        XCTAssertEqual(style.buttonStroke, .ink2)
    }

    func testPanelCloseRoleFollowsFullAccess() {
        XCTAssertEqual(ScopeDisplay.panelCloseRole(fullAccess: false), .ink)
        XCTAssertEqual(ScopeDisplay.panelCloseRole(fullAccess: true), .accent)
    }
}
