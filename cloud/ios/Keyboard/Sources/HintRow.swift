// 候选栏上方的记忆提示行（设计稿 .k-hint）：恋爱、日常选了人且有提示时才出现，没提示不占行（KeyboardModel.hasHintRow）。
// 浅灰绿底、灰绿字（命中的词加粗）、左侧圆点、下沿一条细线；右边是「你写的」灰色小字（卡片来源，不可点）和按钮：
// 匹配提示是向上的圆形箭头（展开对象卡，展开后同一位置的工具栏换成向下箭头），日子 / 约定提醒是「知道了」。

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
                    .font(.system(size: 12.5))
                    .foregroundStyle(Theme.accentInk.color)
                    .lineLimit(1)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
                    .onKeyboardPress { model.openContactCard() }
                if let label = HintText.sourceLabel(for: model.hintSource) {
                    Text(label)
                        .font(.system(size: 11))
                        .foregroundStyle(Theme.ink3)
                }
                if hint.reason == .today {
                    Text("知道了")
                        .font(.system(size: 11.5))
                        .foregroundStyle(Theme.ink3)
                        .padding(.horizontal, 12)
                        .frame(maxHeight: .infinity)
                        .onKeyboardPress { model.acknowledgeHint() }
                } else {
                    ToolbarArrow(up: true, label: "展开对象卡") { model.openContactCard() }
                }
            } else {
                Color.clear.frame(maxWidth: .infinity)
            }
        }
        .frame(height: KeyStyle.hintRowHeight)
        .background(Theme.accentSoft.color.opacity(hint == nil ? 0 : 1))
        .overlay(alignment: .bottom) {
            if hint != nil { Rectangle().fill(Theme.hintLine.color).frame(height: 1) }
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel(hint.map { "记忆提示：\($0.text)" } ?? "")
    }
}
