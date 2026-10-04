// 候选栏上方的记忆提示行：恋爱场景选了对象时一直在（没有提示时是空行，高度不变，宿主界面不跳）。
// 有提示时左边强调色圆点与文字（命中的词加粗），右边是「你写的」灰色小字（卡片来源，不可点）和按钮：匹配提示是「展开」（键区换成对象卡），
// 日子 / 约定提醒是「知道了」。

import SwiftUI

struct HintRow: View {
    let model: KeyboardModel

    let hint: MemoryHint?

    var body: some View {
        HStack(spacing: 8) {
            if let hint {
                Circle()
                    .fill(Theme.accent.color)
                    .frame(width: 6, height: 6)
                    .padding(.leading, 12)
                Text(HintText.attributed(text: hint.text, emphasis: model.hintEmphasis))
                    .font(.system(size: 14))
                    .foregroundStyle(Theme.accentInk.color)
                    .lineLimit(1)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
                    .onKeyboardPress { model.openContactCard() }
                if let label = HintText.sourceLabel(for: model.hintSource) {
                    Text(label)
                        .font(.system(size: 11))
                        .foregroundStyle(.secondary.opacity(0.7))
                }
                Text(hint.reason == .today ? "知道了" : "展开")
                    .font(.system(size: 14, weight: .medium))
                    .foregroundStyle(Theme.accentInk.color)
                    .padding(.trailing, 12)
                    .frame(maxHeight: .infinity)
                    .onKeyboardPress {
                        if hint.reason == .today { model.acknowledgeHint() } else { model.openContactCard() }
                    }
            } else {
                Color.clear.frame(maxWidth: .infinity)
            }
        }
        .frame(height: KeyStyle.hintRowHeight)
        .background(Theme.accentSoft.color.opacity(hint == nil ? 0 : 1))
        .accessibilityElement(children: .contain)
        .accessibilityLabel(hint.map { "记忆提示：\($0.text)" } ?? "")
    }
}
