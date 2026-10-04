// 整个键盘：提示行（恋爱场景选了对象时一直在）+ 候选栏 + 键区（或展开的候选 / 表情 / 场景选择 / 对象卡）。键的位置与触摸范围由 KeyboardLayout 算；
// 进出「恋爱 · 某人」时键盘高度加减一行，高度约束在控制器里改（KeyboardViewController.syncHintRow）。

import SwiftUI

struct KeyboardView: View {
    let model: KeyboardModel

    /// 系统要求自带切换键（没有键盘下方的地球键）时，底行 😀 的位置换成地球键。
    let showsGlobe: Bool

    /// 键区高度：四行键加行距与上下留白，展开面板也占这么高，切换时键盘不跳。
    static let keyAreaHeight = KeyStyle.keyHeight * 4 + KeyStyle.rowSpacing * 3 + KeyboardLayout.topPadding + 6

    var body: some View {
        VStack(spacing: 0) {
            if model.hasHintRow {
                HintRow(model: model, hint: model.hint)
                    .transition(.move(edge: .top).combined(with: .opacity))
            }
            CandidateBar(model: model)
            Group {
                switch model.panel {
                case .keys: keys
                // 面板是控制器挂的 UIKit 视图（CandidatePanelView），这里留空占位
                case .candidates: Color.clear
                case .emoji: EmojiPanel(model: model)
                case .scope: ScopePicker(model: model)
                case .contactCard: ContactCardPanel(model: model)
                }
            }
            .frame(height: Self.keyAreaHeight)
        }
        .frame(maxHeight: .infinity, alignment: .top)
        .animation(.easeOut(duration: 0.2), value: model.hasHintRow)
    }

    private var keys: some View {
        GeometryReader { geometry in
            let slots = KeyboardLayout.slots(layer: model.layer, showsGlobe: showsGlobe, size: geometry.size)
            ZStack(alignment: .topLeading) {
                ForEach(Array(slots.enumerated()), id: \.offset) { index, slot in
                    KeyButton(
                        key: slot.key, model: model, insets: slot.insets,
                        pressed: model.pressedSlots.contains(index))
                        .frame(width: slot.cell.width, height: slot.cell.height)
                        .position(x: slot.cell.midX, y: slot.cell.midY)
                }
            }
        }
    }
}
