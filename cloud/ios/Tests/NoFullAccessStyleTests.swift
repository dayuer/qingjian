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
        XCTAssertEqual(
            accent, [.chip, .hintBackground, .hintDot, .selectedContactCell, .avatar, .noteConfirm, .reminderCard])
    }

    func testButtonsAndPanelControlsAreNeutral() {
        for usage in [ColorUsage.hintButton, .panelDone, .cardClose, .allMemoryButton, .noteIgnore] {
            XCTAssertEqual(usage.role, .ink, "\(usage)")
        }
        XCTAssertEqual(ColorUsage.cardNotice.role, .ink2)
    }

    /// App 的「键盘记住的事」：加人、记一条、存好、设置这些控件，以及读写失败的提示都是中性色，只有今天的提醒卡（关于某个人）用灰绿底。
    func testMemoryAppControlsAreNeutral() {
        for usage in [ColorUsage.addContactButton, .addCardButton, .editorSave, .contactSettingsButton, .cloudIntroLink, .failureBanner] {
            XCTAssertEqual(usage.role, .ink, "\(usage)")
        }
        XCTAssertEqual(ColorUsage.reminderCard.role, .accent)
    }
}
