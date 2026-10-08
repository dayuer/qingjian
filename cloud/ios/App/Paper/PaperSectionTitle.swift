// 设计稿 .sec-t：分组上面的小节标题，500 12 ink-3、字距 0.5、左右 20；右边可选一段常规字重的说明。

import SwiftUI

struct PaperSectionTitle: View {
    let text: String

    var trailing: String?

    var body: some View {
        HStack {
            Text(text)
            Spacer()
            if let trailing {
                Text(trailing).fontWeight(.regular)
            }
        }
        .font(AppFont.font(size: 12, weight: .medium))
        .tracking(0.5)
        .foregroundStyle(Theme.ink3)
        .padding(.horizontal, 20)
    }
}
