// 整个键盘：候选栏 + 键区（或展开的候选 / 表情面板）。键的位置与触摸范围由 KeyboardLayout 算。

import SwiftUI

struct KeyboardView: View {
    let model: KeyboardModel

    /// 系统要求自带切换键（没有键盘下方的地球键）时，底行 😀 的位置换成地球键。
    let showsGlobe: Bool

    /// 键区高度：四行键加行距与上下留白，展开面板也占这么高，切换时键盘不跳。
    static let keyAreaHeight = KeyStyle.keyHeight * 4 + KeyStyle.rowSpacing * 3 + KeyboardLayout.topPadding + 6

    var body: some View {
        VStack(spacing: 0) {
            CandidateBar(model: model)
            Group {
                switch model.panel {
                case .keys: keys
                case .candidates: CandidatePanel(model: model)
                case .emoji: EmojiPanel(model: model)
                }
            }
            .frame(height: Self.keyAreaHeight)
        }
    }

    private var keys: some View {
        GeometryReader { geometry in
            let slots = KeyboardLayout.slots(layer: model.layer, showsGlobe: showsGlobe, size: geometry.size)
            ZStack(alignment: .topLeading) {
                ForEach(Array(slots.enumerated()), id: \.offset) { _, slot in
                    KeyButton(key: slot.key, model: model, insets: slot.insets)
                        .frame(width: slot.cell.width, height: slot.cell.height)
                        .position(x: slot.cell.midX, y: slot.cell.midY)
                }
            }
        }
    }
}
