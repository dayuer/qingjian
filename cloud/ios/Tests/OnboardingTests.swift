// 首次引导：步骤顺序、进度条亮几格、要不要出引导（读失败也出）、文案与键盘面板同源、引导里的颜色照设计稿用灰绿。

import XCTest
@testable import QingjianCloud

@MainActor
final class OnboardingTests: XCTestCase {
    func testStepOrder() {
        XCTAssertEqual(OnboardingStep.allCases, [.intro, .keyboard, .plan, .contact])
        XCTAssertEqual(OnboardingStep.allCases.map(\.number), [1, 2, 3, 4])
        XCTAssertEqual(OnboardingStep.intro.next, .keyboard)
        XCTAssertEqual(OnboardingStep.keyboard.next, .plan)
        XCTAssertEqual(OnboardingStep.plan.next, .contact)
        XCTAssertNil(OnboardingStep.contact.next)
    }

    /// 第 n 步亮 n 格（设计稿 2d 两格、2e 三格）；介绍那一步照 1a 不画进度条。
    func testProgressCells() {
        XCTAssertEqual(OnboardingStep.keyboard.progressCells, [true, true, false, false])
        XCTAssertEqual(OnboardingStep.plan.progressCells, [true, true, true, false])
        XCTAssertEqual(OnboardingStep.contact.progressCells, [true, true, true, true])
        for step in OnboardingStep.allCases {
            XCTAssertEqual(step.progressCells.filter { $0 }.count, step.number, "\(step)")
        }
        XCTAssertFalse(OnboardingStep.intro.showsProgress)
        XCTAssertTrue(OnboardingStep.allCases.dropFirst().allSatisfy(\.showsProgress))
    }

    func testShowsWhenNotSeenAndNoContacts() {
        XCTAssertTrue(OnboardingGate.shouldShow(done: false, contacts: []))
    }

    func testHiddenOnceSeen() {
        XCTAssertFalse(OnboardingGate.shouldShow(done: true, contacts: []))
        XCTAssertFalse(OnboardingGate.shouldShow(done: true, contacts: nil))
    }

    /// 已经有对象的老用户不打扰。
    func testHiddenWhenContactsExist() {
        let contact = MemoryContact.new(name: "小美", pronoun: .taF, scene: "dating")
        XCTAssertFalse(OnboardingGate.shouldShow(done: false, contacts: [contact]))
    }

    /// 读失败、App Group 不可用当作没有对象：照样出引导，不因为读不到就跳过。
    func testReadFailureCountsAsNoContacts() {
        XCTAssertTrue(OnboardingGate.shouldShow(done: false, contacts: nil))
    }

    func testDoneKeyLivesInAppDefaults() {
        XCTAssertEqual(OnboardingGate.doneKey, "onboardingDone")
    }

    /// 完全访问那一行与键盘面板、「我」页同一段话（T2 的 ScopeDisplay）。
    func testKeyboardStepTextsShareSourceWithKeyboard() {
        XCTAssertEqual(KeyboardStep.fullAccessText, ScopeDisplay.needsFullAccessText)
        XCTAssertEqual(KeyboardStep.fullAccessPath, ScopeDisplay.fullAccessPath)
    }

    /// 05 的 2e：主按钮是免费版，界面不出现账号一类的字（UI 清单约束 5）。
    func testPlanStepTexts() {
        XCTAssertEqual(PlanStep.useFree, "先用免费版")
        XCTAssertEqual(PlanStep.learnCloud, "了解云服务")
        for text in PlanStep.allTexts {
            for word in ["账号", "登录", "注册"] {
                XCTAssertFalse(text.contains(word), "「\(text)」里有「\(word)」")
            }
        }
    }

    func testIntroIllustrationSlot() {
        XCTAssertEqual(IntroStep.illustration, "onboarding-intro")
        XCTAssertEqual(IntroStep.illustrationHeight, 260)
    }

    /// 约束 1：App 的配色照设计稿，进度条与对勾是 theme.css 的 var(--accent) 实底。
    func testOnboardingColorsAreAccent() {
        XCTAssertEqual(ColorUsage.onboardingProgress.role, .accentFill)
        XCTAssertEqual(ColorUsage.onboardingStepNumber.role, .ink, "步骤编号不画成已完成")
        XCTAssertEqual(ColorUsage.onboardingCurrentPlan.role, .accent)
        XCTAssertEqual(ColorUsage.onboardingPlanBadge.role, .accentSoft)
    }

    func testReplayEntryTitle() {
        XCTAssertEqual(MeView.replayOnboardingTitle, "重新看引导")
    }
}
