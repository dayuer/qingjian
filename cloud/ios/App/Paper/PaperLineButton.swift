// 设计稿 .btn.line：一圈淡灰细线的胶囊按钮，字用 .btn 的 500 13。1b 的「+ 记一条」高 36、用 accent-ink；
// 2j 的「去开启」高 30、用 ink。放在块里时自己不带外边距，由调用方排。

import SwiftUI

struct PaperLineButton: View {
    let title: String

    var height: CGFloat = 32

    var color: Color = Theme.ink

    let action: () -> Void

    var body: some View {
        Button(action: action) {
            Text(title)
                .font(AppFont.font(size: 13, weight: .medium))
                .foregroundStyle(color)
                .padding(.horizontal, 14)
                .frame(height: height)
                .overlay(Capsule().strokeBorder(Hairline.line))
                .contentShape(Capsule())
        }
        .buttonStyle(.plain)
    }
}
