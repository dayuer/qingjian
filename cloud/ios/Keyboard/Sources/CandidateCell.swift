// 一个候选：首选带灰底、字重 500（和这个人说话时再用灰绿字），大模型给的用次要灰。

import SwiftUI

struct CandidateCell: View {
    let text: String

    let highlighted: Bool

    var cloud = false

    /// 首选用灰绿字（ScopeDisplay.accentFirstCandidate）。
    var accent = false

    let action: () -> Void

    var body: some View {
        Text(text)
            .font(.system(size: 22, weight: highlighted ? .medium : .regular))
            .foregroundStyle(accent ? ColorUsage.firstCandidate.role.color : cloud ? Theme.ink2 : Theme.ink)
            .lineLimit(1)
            .padding(.horizontal, 10)
            .frame(minWidth: 40, minHeight: 40)
            .background {
                if highlighted {
                    RoundedRectangle(cornerRadius: 8).fill(KeyStyle.highlightFill)
                }
            }
            .onKeyboardTap(action)
    }
}
