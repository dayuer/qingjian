// 键盘里不在滚动区里的按钮：按下即触发，与按键一样。
//
// 候选栏右端的 ⌄ 用 Button 或「抬手触发」时，模拟器与真机上短点击都可能收不到抬手（长按才行），
// 按键用的「按下触发」一直可靠，所以照搬。滚动区里的候选用 TapAction（按下触发会挡住滑动）。

import SwiftUI

struct PressAction: ViewModifier {
    let action: () -> Void

    @State private var pressed = false

    func body(content: Content) -> some View {
        content
            .contentShape(Rectangle())
            .opacity(pressed ? 0.5 : 1)
            .gesture(
                DragGesture(minimumDistance: 0)
                    .onChanged { _ in
                        guard !pressed else { return }
                        pressed = true
                        action()
                    }
                    .onEnded { _ in pressed = false }
            )
    }
}

extension View {
    func onKeyboardPress(_ action: @escaping () -> Void) -> some View {
        modifier(PressAction(action: action))
    }
}
