// 键盘与 App 上每个带颜色的元素用哪个角色：灰绿 accent 只给「代表某个人」的元素（牌子的底与人名、提示行底与圆点、头像、
// App 首页关于某人的今日提醒卡）、「记到 {对象}」这个把内容记到某人身上的动作、对象卡页脚的「全部记忆」（设计稿 1b）；
// 键盘的文字按钮与面板控件（.tool、.arw、.btn.ghost）是 ink-2，App 的控件与失败提示是 ink。
// 例外：App 里的开关（appToggle）与「加一个人」的「好了」（addContactDone，设计稿 btn.acc：灰绿底、ink 字）打开时用灰绿底，照设计稿 theme.css 的 .toggle 用 Theme.accent（浅 #C0E7C6、深 #34563B，不是 accentInk）；「控件一律中性色」只管键盘面板。
// 首次引导照设计稿也用灰绿：进度条亮的格子（onboardingProgress，.progress i.on 的 accent 实底；步骤编号是 .step .n 的 ink 底，App 测不到完成与否，不画对勾），
// 免费方案卡的描边与「现在就是」（onboardingCurrentPlan、onboardingPlanBadge，.plan.on / .pill.r）。
// App 对象详情的「待整理」一节设计稿没画（UI 清单约束 7 第 17 条）：原话是素材不是卡，一律中性色，删除与开通引导是 ink / ink2。

enum ColorUsage: CaseIterable {
    /// 牌子的底（关于某个人的，用灰绿浅底）。
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

    /// 改一条的「存好」（设计稿 02 的 1c：`--accent-ink`）。
    case editorConfirm

    /// 改一条的「取消」（设计稿 `.btn.ghost`：ink-2）。
    case editorCancel

    /// 对象详情右上「设置」（设计稿 02 的 1b：`--accent-ink`）。
    case contactSettingsButton

    /// 对象设置里「导出记忆」一行。
    case exportLink
    case cloudIntroLink
    case failureBanner

    /// 「待整理」每条展开后的「删除」。
    case materialDelete

    /// 没开素笺云时「待整理」下面那一行开通引导。
    case materialsCloudLink

    /// 待整理快满（180 条）时对象详情顶上的提示。
    case materialsNudge

    /// 首页日历条里选中那天的底（设计稿 02 的 2a：`--accent-soft`）。
    case calendarSelectedDay

    /// 日历条里「有事那天」的小圆点（设计稿 2a：`--ink`）。
    case calendarEventDot

    /// 功勋路已经亮的点与名字（设计稿 2a：`--ink`）。
    case milestoneEarned

    /// 功勋路「最近的下一站」的点与名字（设计稿 2a：`--accent` / `--accent-ink`）。
    case milestoneNext

    /// 功勋路「下一站」那个点本身（设计稿 2a：`--accent` 实底；名字才是 `--accent-ink`）。
    case milestoneNextDot

    /// 「下一站」那个点外圈的光晕（设计稿 2a：`--accent-soft`）。
    case milestoneNextHalo

    /// 事件行尾的种类标记（日子 / 约定 / 近况）：中性灰字 + 线框。
    case eventKindTag

    /// 事件行尾可点的动作标记（「去确认」「补上」）的底与字（设计稿 `.pill.r`）。
    case eventActionTag
    case eventActionTagInk

    /// 首页右上「+ 记一条」的底（灰绿实底，设计稿 2a）。
    case quickNoteButton

    /// 通讯录行尾的事件提示（设计稿 2b：「有事的人在行尾用浅绿字提示」）。
    case contactEventNote

    /// 草稿卡里「拿不准」那项的说明、虚线与问号圈（设计稿 01 的 1e-2：`--ink-3`；问号字是 ink-2）。
    case draftUnsure

    /// 冲突屏里新卡那圈的描边（设计稿 1e-3：`--accent` 的 1.5px 环）。
    case conflictNewRing

    /// 冲突屏里新卡的时间标签「新的 · 今天」（`--accent-ink`）。
    case conflictNewLabel

    /// 冲突屏里「两条都留」这个线框按钮的字（`.btn.line`）。
    case conflictBoth

    var role: ColorRole {
        switch self {
        case .chipPerson, .hintBackground, .hintDot, .selectedContactCell, .avatar, .reminderDay, .firstCandidate,
             .allMemoryButton, .onboardingCurrentPlan: .accent
        case .appLink: .accent
        case .milestoneNext, .eventActionTagInk, .contactEventNote, .editorConfirm, .contactSettingsButton: .accent
        case .milestoneNextDot: .accentFill
        case .onboardingStepNumber: .ink
        case .milestoneEarned, .calendarEventDot: .ink
        case .reminderCard, .chipBackground, .onboardingPlanBadge: .accentSoft
        case .calendarSelectedDay, .milestoneNextHalo, .eventActionTag: .accentSoft
        case .conflictNewRing: .accentFill
        case .conflictNewLabel: .accent
        case .appToggle, .addContactDone, .noteConfirm, .onboardingProgress, .quickNoteButton: .accentFill
        case .addContactButton, .addCardButton,
             .editorSave, .exportLink, .cloudIntroLink, .failureBanner, .materialsCloudLink, .materialsNudge: .ink
        case .cardNotice, .hintButton, .panelDone, .cardClose, .noteIgnore, .noteCancel, .materialDelete,
             .eventKindTag, .editorCancel: .ink2
        case .draftUnsure: .ink3
        case .conflictBoth: .ink
        }
    }
}
