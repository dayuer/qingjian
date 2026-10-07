// 设计稿 .group：左右外边距 16、白底、圆角 14、外圈一道淡灰细线。行与行之间的细线由调用方放 PaperRowLine
// （iOS 17 没有 Group(subviews:)，取不到子视图挨个插线）。fill 可换成 paper2（2c 的「懒得自己写？」是灰底、无外圈）。

import SwiftUI

struct PaperGroup<Content: View>: View {
    var fill: Color = Theme.paperCard

    var ring = true

    @ViewBuilder let content: Content

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            content
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(RoundedRectangle(cornerRadius: 14).fill(fill))
        .overlay {
            if ring { RoundedRectangle(cornerRadius: 14).strokeBorder(Hairline.ring) }
        }
        .clipShape(RoundedRectangle(cornerRadius: 14))
        .padding(.horizontal, 16)
    }
}
