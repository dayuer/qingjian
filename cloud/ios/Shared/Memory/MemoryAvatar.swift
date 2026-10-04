// 头像字：名字的第一个字放在圆里，选中时底色换成强调色（App 的列表、详情与键盘的对象卡、选择面板共用）。

import SwiftUI

struct MemoryAvatar: View {
    let name: String

    var size: CGFloat = 40

    var selected = false

    var body: some View {
        Text(name.first.map(String.init) ?? "?")
            .font(.system(size: size * 0.45, weight: .semibold))
            .foregroundStyle(Theme.accentInk.color)
            .frame(width: size, height: size)
            .background(Circle().fill(selected ? Theme.accent.color : Theme.accentSoft.color))
            .accessibilityHidden(true)
    }
}
