// 设计稿 .mem：一条记忆的正文（15，行高 1.5），下面可选的 meta 小字（11.5 ink-3）。对象详情里没日子的卡用它。

import SwiftUI

struct PaperMemRow: View {
    let text: String

    var meta: String?

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(text)
                .font(AppFont.font(size: 15))
                .lineSpacing(7.5)
                .foregroundStyle(Theme.ink)
                .multilineTextAlignment(.leading)
            if let meta {
                Text(meta)
                    .font(AppFont.font(size: 11.5))
                    .foregroundStyle(Theme.ink3)
            }
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .contentShape(Rectangle())
    }
}
