// 键盘上的一个键。键位照 iOS 自带的简体拼音 26 键，布局在 KeyLayer。

enum Key: Hashable {
    case letter(Character)

    /// 原样输出的字符；数字层与符号层的中文标点直接印在键上（。，、？！），不再经过引擎转换。
    case symbol(String)

    case shift
    case backspace
    case layer(KeyLayer)

    /// 系统要求自带切换键时（没有键盘下方的地球键）占 😀 的位置。
    case globe

    case emoji
    case space
    case returnKey

    /// 按下时在键上方弹出放大字样（与系统键盘一样，只有字符键弹）。
    var showsCallout: Bool {
        switch self {
        case .letter, .symbol: true
        default: false
        }
    }
}
