// 键盘与 App 上每个带颜色的元素用哪个角色：灰绿 accent 只给「代表某个人」的元素（牌子的人那半边与底、提示行底与圆点、选中的对象格、头像、
// App 首页关于某人的今日提醒卡）、「记到 {对象}」这个把内容记到某人身上的动作、对象卡页脚的「全部记忆」（设计稿 1b）；
// 键盘的文字按钮与面板控件（.tool、.arw、.btn.ghost）是 ink-2，App 的控件与失败提示是 ink。
// 例外：App 里的开关（appToggle）与「加一个人」的「好了」（addContactDone，设计稿 btn.acc：灰绿底、accentInk 字）打开时用灰绿底，照设计稿 theme.css 的 .toggle 用 Theme.accent（浅 #C0E7C6、深 #34563B，不是 accentInk）；「控件一律中性色」只管键盘面板。
// 首次引导照设计稿也用灰绿：进度条亮的格子（onboardingProgress，.progress i.on 的 accent 实底；步骤编号是 .step .n 的 ink 底，App 测不到完成与否，不画对勾），
// 免费方案卡的描边与「现在就是」（onboardingCurrentPlan、onboardingPlanBadge，.plan.on / .pill.r）。
// 工作场景里灰绿全部换成中性色（role(in:)）：牌子两半都是中性色，对象格与头像也是。

enum ColorUsage: CaseIterable {
    /// 牌子左半的场景名（任何场景都是 ink2）。
    case chipScene

    /// 牌子左半的底（恋爱、日常是灰绿浅底）。
    case chipBackground

    /// 牌子右半的圆点与人名。
    case chipPerson

    case hintBackground
    case hintDot
    case selectedContactCell
    case avatar
    /// 「记到 {对象}」与「好了」的底（设计稿 btn.acc：灰绿实底、ink 字）。
    case noteConfirm

    /// 恋爱、日常选了人时的首选候选。
    case firstCandidate
    case reminderCard
    case reminderDay
    case appToggle
    case appLink
    case addContactDone
    case onboardingProgress
    case onboardingStepNumber

    /// 引导里当前方案（免费）卡的 2pt 描边与「现在就是」的字。
    case onboardingCurrentPlan

    /// 「现在就是」的浅底。
    case onboardingPlanBadge

    case hintButton
    case panelDone
    case cardClose
    case allMemoryButton
    case cardNotice
    case noteIgnore
    case noteCancel
    case addContactButton
    case addCardButton
    case editorSave
    case contactSettingsButton
    case cloudIntroLink
    case failureBanner

    var role: ColorRole {
        switch self {
        case .chipPerson, .hintBackground, .hintDot, .selectedContactCell, .avatar, .reminderDay, .firstCandidate,
             .allMemoryButton, .onboardingCurrentPlan: .accent
        case .appLink: .accent
        case .onboardingStepNumber: .ink
        case .reminderCard, .chipBackground, .onboardingPlanBadge: .accentSoft
        case .appToggle, .addContactDone, .noteConfirm, .onboardingProgress: .accentFill
        case .addContactButton, .addCardButton,
             .editorSave, .contactSettingsButton, .cloudIntroLink, .failureBanner: .ink
        case .cardNotice, .chipScene, .hintButton, .panelDone, .cardClose, .noteIgnore, .noteCancel: .ink2
        }
    }

    /// 在 `scene` 场景里用的角色：工作场景把灰绿换成中性色（MemoryScope.usesAccent）。
    func role(in scene: String) -> ColorRole {
        MemoryScope.usesAccent(scene) ? role : role.neutralized
    }
}
