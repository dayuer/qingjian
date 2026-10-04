// 头像字：名字的第一个字放在强调色圆里（App 的列表、详情与键盘的对象卡共用）。

import SwiftUI

struct MemoryAvatar: View {
    let name: String

    var size: CGFloat = 40

    var body: some View {
        Text(name.first.map(String.init) ?? "?")
            .font(.system(size: size * 0.45, weight: .semibold))
            .foregroundStyle(Color.accentColor)
            .frame(width: size, height: size)
            .background(Circle().fill(Color.accentColor.opacity(0.15)))
            .accessibilityHidden(true)
    }
}
