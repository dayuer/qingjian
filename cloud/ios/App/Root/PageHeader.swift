// 三个 Tab 页（记得 · 通讯录 · 我）共用的标题行：大标题在左、动作在右（设计稿第三次导出：
// 标题与按钮合成一行，`padding:14px 20px 6px`）。
//
// 共用一个件是为了高度天然一致——各页自己调间距的话，放进 Form 的那页会被 List 的分节内边距
// 往下推 35pt（踩过：记得 67pt、「我」102pt）。

import SwiftUI

struct PageHeader<Action: View>: View {
    let title: String

    let action: Action

    init(title: String, @ViewBuilder action: () -> Action) {
        self.title = title
        self.action = action()
    }

    var body: some View {
        HStack(spacing: 12) {
            Text(title)
                .font(AppFont.font(size: 30, weight: .semibold))
                .tracking(0.5)
                .foregroundStyle(Theme.ink)
            Spacer(minLength: 0)
            action
        }
        .padding(.horizontal, 20)
        .padding(.top, 14)
        .padding(.bottom, 6)
    }
}

extension PageHeader where Action == EmptyView {
    /// 右边没有动作的页（「我」）。
    init(title: String) {
        self.init(title: title) { EmptyView() }
    }
}
