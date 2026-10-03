// 滚动区里的「点一下」（候选、表情）：抬手时触发，按下时变淡。
// 用 simultaneousGesture 挂，不挡 ScrollView 的滑动；手指挪远了就不算点。不在滚动区里的用 PressAction。

import SwiftUI

struct TapAction: ViewModifier {
    let action: () -> Void

    @State private var pressed = false

    func body(content: Content) -> some View {
        content
            .contentShape(Rectangle())
            .opacity(pressed ? 0.5 : 1)
            .simultaneousGesture(
                DragGesture(minimumDistance: 0)
                    .onChanged { value in pressed = Self.isTap(value) }
                    .onEnded { value in
                        pressed = false
                        if Self.isTap(value) { action() }
                    }
            )
    }

    private static func isTap(_ value: DragGesture.Value) -> Bool {
        abs(value.translation.width) < 10 && abs(value.translation.height) < 10
    }
}

extension View {
    func onKeyboardTap(_ action: @escaping () -> Void) -> some View {
        modifier(TapAction(action: action))
    }
}
