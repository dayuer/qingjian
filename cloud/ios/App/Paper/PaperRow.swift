// 设计稿 .row：左边标题（15）与可选副标题（12.5 ink-3），右边可选的灰字（.side 12 ink-3）或任意控件，
// 最后可选的「›」（.chev）。tight 是 2j 那种紧凑行（上下 10）。titleColor 只有「我」的两行危险操作会改。

import SwiftUI

struct PaperRow<Trailing: View>: View {
    let title: String

    var subtitle: String?

    var side: String?

    var chevron = false

    var tight = false

    var titleColor: Color = Theme.ink

    @ViewBuilder var trailing: Trailing

    var body: some View {
        HStack(spacing: 12) {
            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                    .font(AppFont.font(size: 15))
                    .foregroundStyle(titleColor)
                if let subtitle {
                    Text(subtitle)
                        .font(AppFont.font(size: 12.5))
                        .foregroundStyle(Theme.ink3)
                        .lineSpacing(2)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            if let side {
                Text(side)
                    .font(AppFont.font(size: 12))
                    .foregroundStyle(Theme.ink3)
                    .multilineTextAlignment(.trailing)
            }
            trailing
            if chevron {
                Text("›")
                    .font(AppFont.font(size: 16))
                    .foregroundStyle(Theme.ink3)
                    // 「›」只是个记号：读屏与 UI 测试认这一行时应该只听到标题（不然会读成「键盘设置, ›」）
                    .accessibilityHidden(true)
            }
        }
        .padding(.horizontal, 14)
        .padding(.vertical, tight ? 10 : 12)
        .contentShape(Rectangle())
    }
}

extension PaperRow where Trailing == EmptyView {
    init(
        title: String, subtitle: String? = nil, side: String? = nil, chevron: Bool = false, tight: Bool = false,
        titleColor: Color = Theme.ink
    ) {
        self.init(
            title: title, subtitle: subtitle, side: side, chevron: chevron, tight: tight, titleColor: titleColor
        ) { EmptyView() }
    }
}
