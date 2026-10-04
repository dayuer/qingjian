// 键盘上每个带颜色的元素用哪个角色：灰绿 accent 只给「代表某个人」的元素（牌子、提示行底与圆点、选中的对象格、头像）
// 和「记到 {对象}」这个把内容记到某人身上的动作；按钮与面板控件一律中性色。

enum ColorUsage: CaseIterable {
    case chip
    case hintBackground
    case hintDot
    case selectedContactCell
    case avatar
    case noteConfirm

    case hintButton
    case panelDone
    case cardClose
    case allMemoryButton
    case cardNotice
    case noteIgnore

    var role: ColorRole {
        switch self {
        case .chip, .hintBackground, .hintDot, .selectedContactCell, .avatar, .noteConfirm: .accent
        case .hintButton, .panelDone, .cardClose, .allMemoryButton, .noteIgnore: .ink
        case .cardNotice: .ink2
        }
    }
}
