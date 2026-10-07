// 设计稿 .app + .scroll：白纸底、竖向滚动、各块之间等距（默认 18，「我」用 10）。
// 底部留出 iOS 26 浮动 Tab 栏的高度（RootTab.listBottomMargin），不让最后一块被挡。

import SwiftUI

struct PaperPage<Content: View>: View {
    var spacing: CGFloat = 18

    /// 顶部留白；「我」那一页是 8（设计稿 2j 的 padding-top）。
    var top: CGFloat = 12

    @ViewBuilder let content: Content

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: spacing) {
                content
            }
            .padding(.top, top)
            .padding(.bottom, 20)
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .contentMargins(.bottom, RootTab.listBottomMargin, for: .scrollContent)
        .background(Theme.paper)
    }
}
