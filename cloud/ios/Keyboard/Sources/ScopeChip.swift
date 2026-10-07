// 工具栏左侧的牌子：圆点 + 人名的胶囊（设计稿 1a）。没选人时是「不指定」。
// 点它在工具栏里列出其他人 + 「不指定」（KeyboardModel.toggleQuickPicks）；面板打开时牌子只是标题，点了不做事。
// 没开完全访问时读不到名单，这一块改成说明入口（KeyboardModel.toggleFullAccessNote），点了在工具栏里展开整句与开启路径。

import SwiftUI

struct ScopeChip: View {
    let model: KeyboardModel

    /// 面板打开时牌子只是标题，点了不做事。
    var interactive = true

    var body: some View {
        if ScopeDisplay.chipShowsPerson(fullAccess: model.fullAccess) {
            personChip
        } else {
            fullAccessNote
        }
    }

    /// 没开完全访问：牌子换成说明入口——一行放不下这句，点了展开（IdleBar 的 fullAccessNote）。
    private var fullAccessNote: some View {
        Text(ScopeDisplay.needsFullAccessForRewrite)
            .font(.system(size: 12.5))
            .foregroundStyle(ColorUsage.cardNotice.role.color)
            .lineLimit(1)
            .padding(.horizontal, 10)
            .frame(height: 30)
            .contentShape(Rectangle())
            .onKeyboardPress { if interactive { model.toggleFullAccessNote() } }
            .accessibilityAddTraits(.isButton)
            .padding(.leading, 8)
    }

    private var personChip: some View {
        HStack(spacing: 6) {
            Circle()
                .fill(love ? Theme.accent.color : Color(.tertiaryLabel))
                .frame(width: 6, height: 6)
            Text(ScopeDisplay.chipPerson(model.currentContact?.chipName))
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(ColorUsage.chipPerson.role.color)
                .lineLimit(1)
        }
        .padding(.leading, 9)
        .padding(.trailing, 11)
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
