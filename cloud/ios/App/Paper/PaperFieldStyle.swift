// 设计稿 .field：白底、圆角 12、一圈 --line 细线、内边距 12 / 14、16pt 字。改一条的「记忆」框用它。

import SwiftUI

struct PaperFieldStyle: ViewModifier {
    func body(content: Content) -> some View {
        content
            .font(AppFont.font(size: 16))
            .padding(.horizontal, 14)
            .padding(.vertical, 12)
            .background(RoundedRectangle(cornerRadius: 12).fill(Theme.paperCard))
            .overlay(RoundedRectangle(cornerRadius: 12).strokeBorder(Hairline.line))
    }
}
