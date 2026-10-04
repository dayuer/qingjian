// 没开完全访问那一屏与对象无关：颜色全是中性色，不引用 accent；灰绿只给代表某个人的元素。

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

    func testAccentOnlyOnElementsThatStandForAPerson() {
        let accent = Set(ColorUsage.allCases.filter { $0.role == .accent })
        XCTAssertEqual(accent, [.chip, .hintBackground, .hintDot, .selectedContactCell, .avatar, .noteConfirm])
    }

    func testButtonsAndPanelControlsAreNeutral() {
        for usage in [ColorUsage.hintButton, .panelDone, .cardClose, .allMemoryButton, .noteIgnore] {
            XCTAssertEqual(usage.role, .ink, "\(usage)")
        }
        XCTAssertEqual(ColorUsage.cardNotice.role, .ink2)
    }
}
