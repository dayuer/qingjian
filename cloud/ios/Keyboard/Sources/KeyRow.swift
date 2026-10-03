// 一行键：中间一组字符键，两端可有功能键（⇧ / ⌫ / 切层）。

struct KeyRow: Hashable {
    var leading: Key?

    var middle: [Key]

    var trailing: Key?

    /// 中间的键是否拉宽填满两端键之间（数字层第三行是，字母层 zxcvbnm 保持字母键宽、居中）。
    var stretchesMiddle: Bool

    init(leading: Key?, middle: [Key], trailing: Key?, stretchesMiddle: Bool) {
        self.leading = leading
        self.middle = middle
        self.trailing = trailing
        self.stretchesMiddle = stretchesMiddle
    }

    init(letters: String) {
        self.init(leading: nil, middle: letters.map(Key.letter), trailing: nil, stretchesMiddle: false)
    }

    init(symbols: [String]) {
        self.init(leading: nil, middle: symbols.map(Key.symbol), trailing: nil, stretchesMiddle: false)
    }
}
