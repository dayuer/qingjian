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
        XCTAssertEqual(ScopeDisplay.needsFullAccessText, "开启完全访问后才能用记忆。开了也不联网，卡片只在这台手机上")
        XCTAssertEqual(ScopeDisplay.fullAccessPath, "设置 → 通用 → 键盘 → 键盘 → 素笺 → 允许完全访问")
    }

    /// 没开完全访问时改写也用不了（iOS 键盘扩展没网络），说明要写清「改写和记忆都要它」。
    func testRewriteAlsoNeedsFullAccess() {
        XCTAssertEqual(
            ScopeDisplay.needsFullAccessForRewrite,
            "改写和记忆都要开完全访问。开了也不会上传你没让它上传的内容。")
        XCTAssertFalse(ScopeDisplay.chipShowsPerson(fullAccess: false))
        XCTAssertTrue(ScopeDisplay.chipShowsPerson(fullAccess: true))
    }

    func testAccentOnlyOnElementsThatStandForAPerson() {
        let accent = Set(ColorUsage.allCases.filter { $0.role.isAccent })
        XCTAssertEqual(
            accent,
            [.chipBackground, .chipPerson, .hintBackground, .hintDot, .selectedContactCell, .avatar, .noteConfirm, .reminderCard, .reminderDay,
             .firstCandidate, .allMemoryButton, .appToggle, .addContactDone, .appLink, .onboardingProgress,
             .onboardingCurrentPlan, .onboardingPlanBadge,
             // 首页「记得」（02 的 2a）：日历选中那天、功勋路的下一站、事件行可点的动作标记、「+ 记一条」
             .calendarSelectedDay, .milestoneNext, .milestoneNextDot, .milestoneNextHalo, .eventActionTag, .eventActionTagInk,
             .quickNoteButton,
             // 对象详情右上「设置」与改一条的「存好」（02 的 1b / 1c：App 照设计稿用 --accent-ink）
             .contactSettingsButton, .editorConfirm,
             // 记一笔的草稿卡与冲突屏（01 的 1e-2 / 1e-3）：冲突屏新卡那圈描边与「新的」标签
             .conflictNewRing, .conflictNewLabel,
             // 通讯录行尾的事件提示（02 的 2b：关于某个人的事，用浅绿字）
             .contactEventNote])
    }

    func testButtonsAndPanelControlsAreNeutral() {
        for usage in [ColorUsage.hintButton, .panelDone, .cardClose, .noteIgnore, .noteCancel] {
            XCTAssertEqual(usage.role, .ink2, "\(usage)：键盘的文字按钮是 ink-2（设计稿 .tool / .arw / .btn.ghost）")
        }
        XCTAssertEqual(ColorUsage.cardNotice.role, .ink2)
    }

    /// App 的「键盘记住的事」：加人、记一条、导出这些控件，以及读写失败的提示都是中性色，只有今天的提醒卡（关于某个人）用灰绿底。
    func testMemoryAppControlsAreNeutral() {
        for usage in [ColorUsage.addContactButton, .addCardButton, .editorSave, .exportLink, .cloudIntroLink, .failureBanner,
                      .materialsCloudLink, .materialsNudge] {
            XCTAssertEqual(usage.role, .ink, "\(usage)")
        }
        XCTAssertEqual(ColorUsage.materialDelete.role, .ink2, "「待整理」设计稿没画，原话不是卡，一律中性色")
        XCTAssertEqual(ColorUsage.editorCancel.role, .ink2, "改一条的「取消」是 .btn.ghost")
        XCTAssertEqual(ColorUsage.draftUnsure.role, .ink3, "草稿卡拿不准的说明与虚线是 --ink-3")
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

    /// 没开完全访问时牌子只剩说明入口、整块中性色；开了照常（是给人用的，用灰绿）。
    func testChipWithoutFullAccessIsNeutral() {
        XCTAssertFalse(ScopeDisplay.chipShowsPerson(fullAccess: false))
        XCTAssertTrue(ScopeDisplay.chipShowsPerson(fullAccess: true))
        XCTAssertFalse(ScopeDisplay.chipUsesAccent(fullAccess: false))
        XCTAssertTrue(ScopeDisplay.chipUsesAccent(fullAccess: true))
    }

    /// 「我」页那一行是素笺云服务，两种状态各有各的说法，界面里不出现「账号」。
    func testTheCloudRowSaysOpenedOrNot() {
        XCTAssertEqual(MeView.cloudTitle, "素笺云服务")
        XCTAssertEqual(MeView.cloudStatus(signedIn: true), "已开通")
        XCTAssertEqual(MeView.cloudStatus(signedIn: false), "没开通")
        XCTAssertEqual(MeView.cloudStatus(signedIn: nil), "没开通", "还没读出来时按没开通显示")
        for text in [MeView.cloudTitle, MeView.cloudStatus(signedIn: true), MeView.cloudStatus(signedIn: false)] {
            XCTAssertFalse(text.contains("账号"))
        }
    }

    /// 「我」页的说明不分状态：不能写「开启后才能用」，开了的人会以为自己没开。
    func testAppExplanationIsStateless() {
        XCTAssertEqual(ScopeDisplay.fullAccessExplanation, "「完全访问」用于按键震动，以及让键盘读到你在「记得」里写下的人与事。开了也不联网，卡片只在这台手机上。")
        XCTAssertFalse(ScopeDisplay.fullAccessExplanation.contains("登录"), "界面里不出现登录（约束 5）")
        XCTAssertEqual(ColorUsage.appLink.role, .accent, "App 链接色照设计稿 accent-ink")
        XCTAssertFalse(ScopeDisplay.fullAccessExplanation.contains("后才能"))
    }
}
