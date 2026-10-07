// 设计稿 .lbl：表单里输入框上面的小标签，500 12 ink-3。

import SwiftUI

struct PaperLabel: View {
    let text: String

    var body: some View {
        Text(text)
            .font(AppFont.font(size: 12, weight: .medium))
            .foregroundStyle(Theme.ink3)
    }
}
