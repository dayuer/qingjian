// 改写技能排：工具栏那颗技能按钮点开的那一行，改写条（等结果 / 结果 / 没成功）上也带着它。
// 第一项是「用默认」（把人身上的指定清掉）；选一项就切过去并记下来。算当前技能、排选项在 RewriteSkill 里（可单测）。

import SwiftUI

struct RewriteSkillRow: View {
    let skills: [Skill]

    /// 选中的人身上指定的那个（nil = 用默认）；决定哪一项高亮。
    let current: String?

    let onPick: (String?) -> Void

    var body: some View {
        HStack(spacing: 6) {
            ForEach(RewriteSkill.options(skills: skills, current: current)) { option in
                Text(option.title)
                    .font(.system(size: 13))
                    .foregroundStyle(option.selected ? Theme.ink : Theme.ink2)
                    .lineLimit(1)
                    .padding(.horizontal, 10)
                    .frame(height: 30)
                    .background(Capsule().fill(KeyStyle.keyFill))
                    .overlay(
                        Capsule().stroke(
                            option.selected ? Theme.accent.color : Color.clear, lineWidth: 1.5))
                    .contentShape(Rectangle())
                    .onKeyboardPress { onPick(option.id) }
                    .accessibilityAddTraits(option.selected ? [.isButton, .isSelected] : .isButton)
            }
            Spacer(minLength: 0)
        }
        .padding(.horizontal, 12)
    }
}
