// UI 测试按坐标点字母键用：390pt 宽的屏上三行字母键的中心 x，量法同键盘的 KeyStyle / KeyboardLayout。
// 拆成带类型的小步算：写成一个长表达式时 Swift 类型检查超时（「unable to type-check this expression in reasonable time」）。

import CoreGraphics

enum KeyGeometry {
    static let rows = ["qwertyuiop", "asdfghjkl", "zxcvbnm"]

    static let screenWidth: CGFloat = 390

    static let sideMargin: CGFloat = 4

    static let keySpacing: CGFloat = 6.3

    /// ⇧ / ⌫ 的宽度，以字母键宽为 1。
    static let edgeUnits: CGFloat = 1.36

    /// 第二、三行 ⇧ 与 ⌫ 之间那一段的宽度，字母键在里面居中。
    static let middleWidth: CGFloat = 280.92

    /// 字母键宽：一行 10 个键加两侧边距正好铺满。
    static var unit: CGFloat {
        let gaps: CGFloat = keySpacing * 9
        let usable: CGFloat = screenWidth - sideMargin * 2 - gaps
        return usable / 10
    }

    /// 第 `row` 行（0/1/2）第一个字母键的左边。
    static func rowStart(_ row: Int) -> CGFloat {
        guard row > 0 else { return sideMargin }
        let count = CGFloat(rows[row].count)
        let letters: CGFloat = unit * count + keySpacing * (count - 1)
        let afterShift: CGFloat = sideMargin + unit * edgeUnits + keySpacing
        return afterShift + (middleWidth - letters) / 2
    }

    /// `key` 在第 `row` 行的中心 x；不在这一行为 nil。
    static func centerX(_ key: Character, row: Int) -> CGFloat? {
        guard let index = Array(rows[row]).firstIndex(of: key) else { return nil }
        let step: CGFloat = unit + keySpacing
        return rowStart(row) + CGFloat(index) * step + unit / 2
    }

    /// `key` 在哪一行与它的中心 x。
    static func locate(_ key: Character) -> (row: Int, x: CGFloat)? {
        for row in rows.indices {
            if let x = centerX(key, row: row) { return (row, x) }
        }
        return nil
    }
}
