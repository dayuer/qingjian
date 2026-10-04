// 候选栏上方的记忆提示行：恋爱场景选了对象时一直在（没有提示时是空行，高度不变，宿主界面不跳）。
// 有提示时左边强调色圆点与文字，右边匹配提示是「展开」（键区换成对象卡），日子 / 约定提醒是「知道了」。

import SwiftUI

struct HintRow: View {
    let model: KeyboardModel

    let hint: MemoryHint?

    var body: some View {
        HStack(spacing: 8) {
            if let hint {
                Circle()
                    .fill(Color.accentColor)
                    .frame(width: 6, height: 6)
                    .padding(.leading, 12)
                Text(hint.text)
                    .font(.system(size: 14))
                    .lineLimit(1)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
                    .onKeyboardPress { model.openContactCard() }
                Text(hint.reason == .today ? "知道了" : "展开")
                    .font(.system(size: 14, weight: .medium))
                    .foregroundStyle(Color.accentColor)
                    .padding(.horizontal, 12)
                    .frame(maxHeight: .infinity)
                    .onKeyboardPress {
                        if hint.reason == .today { model.acknowledgeHint() } else { model.openContactCard() }
                    }
            } else {
                Color.clear.frame(maxWidth: .infinity)
            }
        }
        .frame(height: KeyStyle.hintRowHeight)
        .background(Color.accentColor.opacity(hint == nil ? 0 : 0.06))
        .accessibilityElement(children: .contain)
        .accessibilityLabel(hint.map { "记忆提示：\($0.text)" } ?? "")
    }
}
