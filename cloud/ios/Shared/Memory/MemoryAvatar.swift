// 头像字：名字的第一个字放在圆里，选中时底色换成强调色（App 的列表、详情与键盘的对象卡、选择面板共用）。
// 工作场景的人用中性色（MemoryScope.usesAccent）。

import SwiftUI

struct MemoryAvatar: View {
    let name: String

    var size: CGFloat = 40

    var selected = false

    var scene = MemoryScope.dating

    /// 设计稿 .avatar 的衬线字（键盘里的对象卡与选择面板用）。
    var serif = false

    /// 只有选中时才用灰绿（设计稿 1d 的格子：没选中的头像是灰底 .avatar，选中的是 .avatar.r）。
    var accentOnlyWhenSelected = false

    var body: some View {
        Text(name.first.map(String.init) ?? "?")
            .font(.system(size: size * (serif ? 0.4 : 0.45), weight: serif ? .medium : .semibold, design: serif ? .serif : .default))
            .foregroundStyle(plain ? Theme.ink2 : ColorUsage.avatar.role(in: scene).color)
            .frame(width: size, height: size)
            .background(Circle().fill(fill))
            .accessibilityHidden(true)
    }

    private var plain: Bool { !MemoryScope.usesAccent(scene) || (accentOnlyWhenSelected && !selected) }

    private var fill: Color {
        guard !plain else { return ColorRole.neutralSoft.color }
        return selected ? Theme.accent.color : Theme.accentSoft.color
    }
}
