// 键盘与 App 上每个带颜色的元素用哪个角色：灰绿 accent 只给「代表某个人」的元素（牌子、提示行底与圆点、选中的对象格、头像、
// App 首页关于某人的今日提醒卡）和「记到 {对象}」这个把内容记到某人身上的动作；按钮、面板控件与失败提示一律中性色。

enum ColorUsage: CaseIterable {
    case chip
    case hintBackground
    case hintDot
    case selectedContactCell
    case avatar
    case noteConfirm
    case reminderCard

    case hintButton
    case panelDone
    case cardClose
    case allMemoryButton
    case cardNotice
    case noteIgnore
    case addContactButton
    case addCardButton
    case editorSave
    case contactSettingsButton
    case cloudIntroLink
    case failureBanner

    var role: ColorRole {
        switch self {
        case .chip, .hintBackground, .hintDot, .selectedContactCell, .avatar, .noteConfirm, .reminderCard: .accent
        case .hintButton, .panelDone, .cardClose, .allMemoryButton, .noteIgnore, .addContactButton, .addCardButton,
             .editorSave, .contactSettingsButton, .cloudIntroLink, .failureBanner: .ink
        case .cardNotice: .ink2
        }
    }
}
