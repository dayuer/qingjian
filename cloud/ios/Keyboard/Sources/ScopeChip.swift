// 工具栏左侧的牌子（设计稿 .chip.split）：左半场景名、右半圆点加人名（没选人是「不指定」）。
// 恋爱、日常整块用 love 配色（浅灰绿底、灰绿字），右半是白底；工作整块白底、圆点灰色，一点强调色都不出。
// 点左半进选择面板，点右半在工具栏里横列本场景的其他人（KeyboardModel.toggleQuickPicks）；面板打开时只显示、不响应。
// 对象卡顶部用不拆开的那种（split 为假）：只有圆点加名字。

import SwiftUI

struct ScopeChip: View {
    let model: KeyboardModel

    var split = true

    /// 面板打开时牌子只是标题，点了不做事。
    var interactive = true

    var body: some View {
        HStack(spacing: 0) {
            if split {
                Text(ScopeDisplay.chipScene(model.scope.scene))
                    .font(.system(size: 13))
                    .foregroundStyle(ColorUsage.chipScene.role.color)
                    .padding(.leading, 11)
                    .padding(.trailing, 9)
                    .frame(maxHeight: .infinity)
                    .overlay(alignment: .trailing) {
                        Rectangle().fill(love ? Theme.chipLine.color : Color(.separator)).frame(width: 1)
                    }
                    .onKeyboardPress { if interactive { model.openScopePicker() } }
                    .accessibilityAddTraits(.isButton)
                    .accessibilityLabel("场景：\(ScopeDisplay.chipScene(model.scope.scene))")
            }
            person
        }
        .frame(height: 30)
        .background(love ? Theme.accentSoft.color : KeyStyle.keyFill)
        .clipShape(Capsule())
        .shadow(color: .black.opacity(0.08), radius: 0, y: 1)
        .padding(.leading, 8)
    }

    private var person: some View {
        HStack(spacing: 6) {
            Circle()
                .fill(love ? Theme.accent.color : Theme.ink3)
                .frame(width: 6, height: 6)
            Text(ScopeDisplay.chipPerson(model.currentContact?.name))
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(love ? ColorUsage.chipPerson.role.color : Theme.ink)
                .lineLimit(1)
        }
        .padding(.leading, 9)
        .padding(.trailing, 11)
        .frame(maxHeight: .infinity)
        .background(split ? KeyStyle.keyFill : Color.clear)
        .onKeyboardPress { if interactive && split { model.toggleQuickPicks() } }
        .accessibilityAddTraits(.isButton)
        .accessibilityLabel("对象：\(ScopeDisplay.chipPerson(model.currentContact?.name))")
    }

    private var love: Bool { MemoryScope.usesAccent(model.scope.scene) }
}
