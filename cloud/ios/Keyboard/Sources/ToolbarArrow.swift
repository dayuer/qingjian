// 右端的圆形箭头（设计稿 .arw：28pt、ink-2）：工具栏里收起键盘或对象卡（向下），提示行里展开对象卡（向上）。

import SwiftUI

struct ToolbarArrow: View {
    let up: Bool

    let label: String

    let action: () -> Void

    var body: some View {
        Image(systemName: up ? "chevron.up" : "chevron.down")
            .font(.system(size: 13, weight: .semibold))
            .foregroundStyle(Theme.ink2)
            .frame(width: 28, height: 28)
            .contentShape(Circle())
            .padding(.horizontal, 6)
            .frame(maxHeight: .infinity)
            .onKeyboardPress(action)
            .accessibilityAddTraits(.isButton)
            .accessibilityLabel(label)
    }
}
