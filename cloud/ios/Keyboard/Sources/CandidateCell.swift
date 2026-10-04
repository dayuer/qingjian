// 一个候选：候选栏与展开面板共用，首选带灰底，大模型给的用次要灰。

import SwiftUI

struct CandidateCell: View {
    let text: String

    let highlighted: Bool

    var cloud = false

    let action: () -> Void

    var body: some View {
        Text(text)
            .font(.system(size: 22))
            .foregroundStyle(cloud ? Theme.ink2 : Theme.ink)
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
