// 一个按键的外观：按下变色，字符键弹出放大字样。触摸由 KeyTouchView 收，这里只画。
// 格子含四周的缝，键帽按 insets 缩进画在里面。

import SwiftUI

struct KeyButton: View {
    let key: Key

    let model: KeyboardModel

    /// 键帽相对触摸范围的缩进；不给就是键帽占满。
    var insets = EdgeInsets()

    var pressed = false

    var body: some View {
        RoundedRectangle(cornerRadius: KeyStyle.cornerRadius)
            .fill(pressed && !key.showsCallout ? KeyStyle.pressedFill : KeyStyle.keyFill)
            .shadow(color: .black.opacity(0.18), radius: 0, x: 0, y: 1)
            .overlay { label }
            .overlay(alignment: .bottom) {
                if pressed && key.showsCallout { callout }
            }
            .padding(insets)
            .zIndex(pressed ? 1 : 0)
            .accessibilityLabel(accessibilityText)
            .accessibilityAddTraits(.isKeyboardKey)
    }

    @ViewBuilder
    private var label: some View {
        switch key {
        case .letter(let letter):
            Text(model.shifted ? letter.uppercased() : String(letter))
                .font(.system(size: 24))
        case .symbol(let text):
            Text(text).font(.system(size: 22))
        case .shift:
            Image(systemName: model.shifted ? "shift.fill" : "shift")
                .font(.system(size: 19, weight: .medium))
        case .backspace:
            Image(systemName: "delete.left").font(.system(size: 19))
        case .layer(let target):
            Text(target.switchLabel).font(.system(size: 17))
        case .globe:
            Image(systemName: "globe").font(.system(size: 21))
        case .emoji:
            Image(systemName: "face.smiling").font(.system(size: 22))
        case .space:
            // 右下角的「拼」标出当前输入方式
            Text("拼")
                .font(.system(size: 11))
                .foregroundStyle(.secondary)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottomTrailing)
                .padding(.trailing, 10)
                .padding(.bottom, 4)
        case .returnKey:
            Image(systemName: "return").font(.system(size: 20))
        }
    }

    /// 放大字样：比键宽一些，底边连着键帽，往上伸出一个键高。
    private var callout: some View {
        GeometryReader { geometry in
            let width = geometry.size.width + 18
            RoundedRectangle(cornerRadius: KeyStyle.cornerRadius + 2)
                .fill(KeyStyle.keyFill)
                .shadow(color: .black.opacity(0.25), radius: 2, x: 0, y: 1)
                .frame(width: width, height: geometry.size.height * 2.1)
                .overlay(alignment: .top) {
                    calloutText.padding(.top, 8)
                }
                .offset(x: -9, y: -geometry.size.height * 1.1)
        }
        .allowsHitTesting(false)
    }

    @ViewBuilder
    private var calloutText: some View {
        switch key {
        case .letter(let letter):
            Text(model.shifted ? letter.uppercased() : String(letter)).font(.system(size: 36))
        case .symbol(let text):
            Text(text).font(.system(size: 32))
        default:
            EmptyView()
        }
    }

    private var accessibilityText: String {
        switch key {
        case .letter(let letter): String(letter)
        case .symbol(let text): text
        case .shift: "Shift"
        case .backspace: "删除"
        case .layer(let target): target.switchLabel
        case .globe: "下一个键盘"
        case .emoji: "表情符号"
        case .space: "空格"
        case .returnKey: "换行"
        }
    }
}
