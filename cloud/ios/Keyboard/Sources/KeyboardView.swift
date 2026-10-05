// 整个键盘：提示行（恋爱、日常选了人有提示，或记一笔条、手写条时才有）+ 候选栏 + 键区（或展开的候选 / 表情 / 场景选择 / 对象卡）。键的位置与触摸范围由 KeyboardLayout 算；
// 提示行出现与收起时键盘高度加减一行，高度约束在控制器里改（KeyboardViewController.syncHintRow）。
// 对象卡打开时不画提示行（设计稿 1b：顶上是工具栏），那一行的高度给对象卡，键盘总高不变、宿主不跳。

import SwiftUI

struct KeyboardView: View {
    let model: KeyboardModel

    /// 系统要求自带切换键（没有键盘下方的地球键）时，底行 😀 的位置换成地球键。
    let showsGlobe: Bool

    /// 键区高度：四行键加行距与上下留白，展开面板也占这么高，切换时键盘不跳。
    static let keyAreaHeight = KeyStyle.keyHeight * 4 + KeyStyle.rowSpacing * 3 + KeyboardLayout.topPadding + 6

    var body: some View {
        VStack(spacing: 0) {
            if model.hasHintRow && !cardOpen {
                Group {
                    if let composer = model.composedNote {
                        NoteComposeBar(model: model, composer: composer)
                    } else if model.noteDraft != nil || model.noteDone {
                        NoteBar(model: model)
                    } else {
                        HintRow(model: model, hint: model.hint)
                    }
                }
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
                case .draft: DraftPanel(model: model)
                case .conflict: ConflictPanel(model: model)
                }
            }
            .frame(height: Self.keyAreaHeight + (cardOpen && model.hasHintRow ? KeyStyle.hintRowHeight : 0))
        }
        .frame(maxHeight: .infinity, alignment: .top)
        .animation(.easeOut(duration: 0.2), value: model.hasHintRow)
    }

    private var cardOpen: Bool { model.panel == .contactCard }

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
