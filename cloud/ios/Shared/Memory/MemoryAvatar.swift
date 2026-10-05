// 头像字：名字的**最后一个**字放在圆里，选中时底色换成强调色（App 的列表、详情与键盘的对象卡、选择面板共用）。
// 取末字是照设计稿：稿里小美→「美」、阿林→「林」（四份稿子一致）；取首字会让人人都是「小」「阿」。
// 工作场景的人用中性色（MemoryScope.usesAccent）。

import SwiftUI

struct MemoryAvatar: View {
    let name: String

    var size: CGFloat = 40

    var selected = false

    var scene = MemoryScope.dating

    /// 设计稿 .avatar 的衬线字（键盘里的对象卡与选择面板用）。
    var serif = false

    /// 调用方给了字体就用它（主 App 传 MiSans）；键盘不传，照旧系统字体或衬线，不依赖 App 才有的字体。
    var font: Font?

    /// 只有选中时才用灰绿（设计稿 1d 的格子：没选中的头像是灰底 .avatar，选中的是 .avatar.r）。
    var accentOnlyWhenSelected = false

    var body: some View {
        Text(name.last.map(String.init) ?? "?")
            .font(font ?? (serif ? SerifFont.font(size: size * 0.4, weight: .medium) : .system(size: size * 0.45, weight: .semibold)))
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
