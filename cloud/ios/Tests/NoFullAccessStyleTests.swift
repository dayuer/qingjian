// 没开完全访问那一屏与对象无关：颜色全是中性色，不引用 accent；灰绿只给代表某个人的元素。

import UIKit
import XCTest
@testable import QingjianCloud

final class NoFullAccessStyleTests: XCTestCase {
    func testNoFullAccessScreenHasNoAccent() {
        let roles = NoFullAccessStyle.standard.roles
        XCTAssertFalse(roles.contains(.accent))
        XCTAssertFalse(roles.contains { $0.isAccent }, "这一屏与对象无关，灰绿三档一个都不能用")
    }

    func testButtonIsOutlinedWithNeutralColors() {
        let style = NoFullAccessStyle.standard
        XCTAssertEqual(style.closeText, .ink)
        XCTAssertEqual(style.explanation, .ink2)
        XCTAssertEqual(style.buttonText, .ink)
        XCTAssertEqual(style.buttonStroke, .ink2)
        XCTAssertEqual(style.path, .ink)
    }

    /// 审核指南 4.4.1：不开完全访问时只给说明，「去开启」只展开文字路径（键盘扩展不能跳设置、不能借响应链开 App）。
    func testNoFullAccessTexts() {
        XCTAssertEqual(ScopeDisplay.pickerMode(fullAccess: false), .needsFullAccess)
        XCTAssertEqual(ScopeDisplay.needsFullAccessText, "开启完全访问后才能用记忆。开了也不联网，卡片只在这台手机上")
        XCTAssertEqual(ScopeDisplay.fullAccessPath, "设置 → 通用 → 键盘 → 键盘 → 素笺 → 允许完全访问")
    }

    func testAccentOnlyOnElementsThatStandForAPerson() {
        let accent = Set(ColorUsage.allCases.filter { $0.role.isAccent })
        XCTAssertEqual(
            accent,
            [.chipBackground, .chipPerson, .hintBackground, .hintDot, .selectedContactCell, .avatar, .noteConfirm, .reminderCard, .reminderDay,
             .firstCandidate, .allMemoryButton, .appToggle, .addContactDone, .appLink, .onboardingProgress,
             .onboardingCurrentPlan, .onboardingPlanBadge])
    }

    func testButtonsAndPanelControlsAreNeutral() {
        for usage in [ColorUsage.hintButton, .panelDone, .cardClose, .noteIgnore, .noteCancel] {
            XCTAssertEqual(usage.role, .ink2, "\(usage)：键盘的文字按钮是 ink-2（设计稿 .tool / .arw / .btn.ghost）")
        }
        XCTAssertEqual(ColorUsage.cardNotice.role, .ink2)
    }

    /// App 的「键盘记住的事」：加人、记一条、存好、设置这些控件，以及读写失败的提示都是中性色，只有今天的提醒卡（关于某个人）用灰绿底。
    func testMemoryAppControlsAreNeutral() {
        for usage in [ColorUsage.addContactButton, .addCardButton, .editorSave, .contactSettingsButton, .cloudIntroLink, .failureBanner] {
            XCTAssertEqual(usage.role, .ink, "\(usage)")
        }
        XCTAssertEqual(ColorUsage.reminderCard.role, .accentSoft)
        XCTAssertEqual(ColorUsage.reminderDay.role, .accent)
    }

    /// App 的开关照设计稿 .toggle 用灰绿实底（浅 #C0E7C6、深 #34563B），是「控件中性色」的例外，那条只管键盘面板。
    func testAppTogglesUseAccentFill() {
        XCTAssertEqual(ColorUsage.appToggle.role, .accentFill)
        XCTAssertEqual(ColorUsage.addContactDone.role, .accentFill, "「好了」照设计稿 btn.acc 用灰绿底")
        XCTAssertEqual(Theme.accent.light.hex, 0xC0E7C6)
        XCTAssertEqual(Theme.accent.dark.hex, 0x34563B)
    }

    /// 标签栏照设计稿 .tab：选中 ink、未选中 ink-3，不用系统蓝。
    func testTabBarUsesInkNotSystemBlue() {
        XCTAssertEqual(TabBarStyle.selected, UIColor.label)
        XCTAssertEqual(TabBarStyle.normal, UIColor.tertiaryLabel)
    }

    /// 没开完全访问时牌子只剩场景名、整块中性色；开了照常（恋爱、日常灰绿，工作中性）。
    func testChipWithoutFullAccessIsSceneOnlyAndNeutral() {
        XCTAssertFalse(ScopeDisplay.chipShowsPerson(fullAccess: false))
        XCTAssertTrue(ScopeDisplay.chipShowsPerson(fullAccess: true))
        for scene in [MemoryScope.dating, MemoryScope.daily, MemoryScope.work] {
            XCTAssertFalse(ScopeDisplay.chipUsesAccent(scene: scene, fullAccess: false), scene)
        }
        XCTAssertTrue(ScopeDisplay.chipUsesAccent(scene: MemoryScope.dating, fullAccess: true))
        XCTAssertTrue(ScopeDisplay.chipUsesAccent(scene: MemoryScope.daily, fullAccess: true))
        XCTAssertFalse(ScopeDisplay.chipUsesAccent(scene: MemoryScope.work, fullAccess: true))
    }

    /// 界面里不出现账号：「我」页的账号入口藏着，T9 换成开通流程。
    func testAccountEntryIsHidden() {
        XCTAssertFalse(MeView.showsAccountEntry)
    }

    /// 「我」页的说明不分状态：不能写「开启后才能用」，开了的人会以为自己没开。
    func testAppExplanationIsStateless() {
        XCTAssertEqual(ScopeDisplay.fullAccessExplanation, "「完全访问」用于按键震动，以及让键盘读到你在「记得」里写下的人与事。开了也不联网，卡片只在这台手机上。")
        XCTAssertFalse(ScopeDisplay.fullAccessExplanation.contains("登录"), "界面里不出现登录（约束 5）")
        XCTAssertEqual(ColorUsage.appLink.role, .accent, "App 链接色照设计稿 accent-ink")
        XCTAssertFalse(ScopeDisplay.fullAccessExplanation.contains("后才能"))
    }
}
