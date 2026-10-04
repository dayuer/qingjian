// 候选栏左侧的场景牌子：恋爱场景是强调色的「小美 · 恋爱」，日常 / 工作只是灰色场景名（强调色只跟对象有关）。点它打开选择面板。

import SwiftUI

struct ScopeChip: View {
    let model: KeyboardModel

    var body: some View {
        Text(title)
            .font(.system(size: 14, weight: dating ? .semibold : .regular))
            .foregroundStyle(dating ? Theme.accentInk.color : Color.secondary)
            .lineLimit(1)
            .padding(.horizontal, 10)
            .padding(.vertical, 5)
            .background(Capsule().fill(dating ? Theme.accentSoft.color : Color.secondary.opacity(0.12)))
            .padding(.leading, 8)
            .frame(maxHeight: .infinity)
            .onKeyboardPress { model.openScopePicker() }
            .accessibilityAddTraits(.isButton)
            .accessibilityLabel("场景：\(title)")
    }

    private var dating: Bool { model.scope.scene == MemoryScope.dating }

    private var title: String {
        ScopeDisplay.chipTitle(scene: model.scope.scene, contactName: model.currentContact?.name)
    }
}
