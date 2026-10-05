// 工具栏左侧的牌子：圆点 + 人名的胶囊（设计稿 1a）。没选人时是「不指定」。
// 点它在工具栏里列出其他人 + 「不指定」（KeyboardModel.toggleQuickPicks）；面板打开时牌子只是标题，点了不做事。
// 没开完全访问时读不到名单，这一块改成说明入口。

import SwiftUI

struct ScopeChip: View {
    let model: KeyboardModel

    /// 面板打开时牌子只是标题，点了不做事。
    var interactive = true

    var body: some View {
        personChip
    }

    private var personChip: some View {
        HStack(spacing: 6) {
            Circle()
                .fill(love ? Theme.accent.color : Color(.tertiaryLabel))
                .frame(width: 7, height: 7)
            Text(ScopeDisplay.chipPerson(model.currentContact?.chipName))
                .font(.system(size: 13))
                .foregroundStyle(ColorUsage.chipPerson.role.color)
                .lineLimit(1)
        }
        .padding(.leading, 11)
        .padding(.trailing, 12)
        .frame(height: 30)
        .background(love ? Theme.accentSoft.color : KeyStyle.keyFill)
        .clipShape(Capsule())
        .shadow(color: .black.opacity(0.08), radius: 0, y: 1)
        .contentShape(Rectangle())
        .onKeyboardPress { if interactive { model.toggleQuickPicks() } }
        .accessibilityAddTraits(.isButton)
        .accessibilityLabel("对象：\(ScopeDisplay.chipPerson(model.currentContact?.chipName))")
        .padding(.leading, 8)
    }

    private var love: Bool { ScopeDisplay.chipUsesAccent(fullAccess: model.fullAccess) }
}
