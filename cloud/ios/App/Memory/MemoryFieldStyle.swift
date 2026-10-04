// 设计稿的 .field：白底、12 圆角、细描边的输入框。

import SwiftUI

struct MemoryFieldStyle: ViewModifier {
    func body(content: Content) -> some View {
        content
            .padding(.horizontal, 14)
            .padding(.vertical, 12)
            .background(Color(.secondarySystemGroupedBackground), in: RoundedRectangle(cornerRadius: 12))
            .overlay(RoundedRectangle(cornerRadius: 12).strokeBorder(Color(.separator).opacity(0.5)))
    }
}
