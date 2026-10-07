// 设计稿 .group：左右外边距 16、白底、圆角 14、外圈一道淡灰细线。行与行之间的细线由调用方放 PaperRowLine
// （iOS 17 没有 Group(subviews:)，取不到子视图挨个插线）。fill 可换成 paper2，ring 可去掉（灰底那种）。
// 弹层里的分组贴着自己那圈 20 的内边距，inset 传 0（设计稿 1c 的 style="margin:0"）。

import SwiftUI

struct PaperGroup<Content: View>: View {
    var fill: Color = Theme.paperCard

    var ring = true

    var inset: CGFloat = 16

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
        .padding(.horizontal, inset)
    }
}
