// 三层键位：字母、123（数字与中文标点）、#+=（更多符号），逐键对照 iOS 26 简体拼音键盘。

enum KeyLayer: Hashable {
    case letters
    case digits
    case symbols

    /// 上面三行；底行（切层 / 😀 / 空格 / 换行）各层一样，由 KeyboardView 拼。
    var rows: [KeyRow] {
        switch self {
        case .letters:
            [
                KeyRow(letters: "qwertyuiop"),
                KeyRow(letters: "asdfghjkl"),
                KeyRow(leading: .shift, middle: "zxcvbnm".map(Key.letter), trailing: .backspace,
                       stretchesMiddle: false),
            ]
        case .digits:
            [
                KeyRow(symbols: ["1", "2", "3", "4", "5", "6", "7", "8", "9", "0"]),
                KeyRow(symbols: ["-", "/", ":", ";", "(", ")", "¥", "@", "“", "”"]),
                KeyRow(leading: .layer(.symbols), middle: ["。", "，", "、", "？", "！", "."].map(Key.symbol),
                       trailing: .backspace, stretchesMiddle: true),
            ]
        case .symbols:
            [
                KeyRow(symbols: ["【", "】", "{", "}", "#", "%", "^", "*", "+", "="]),
                KeyRow(symbols: ["_", "—", "\\", "|", "～", "《", "》", "$", "&", "·"]),
                KeyRow(leading: .layer(.digits), middle: ["…", "，", "^_^", "？", "！", "’"].map(Key.symbol),
                       trailing: .backspace, stretchesMiddle: true),
            ]
        }
    }

    /// 底行左下角回到别的层的键上印的字。
    var switchLabel: String {
        switch self {
        case .letters: "拼音"
        case .digits: "123"
        case .symbols: "#+="
        }
    }
}
