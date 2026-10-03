// 键位的几何：先按 iOS 简体拼音键盘的尺寸排出每个键帽的位置，再把整块键区分给各个键当触摸范围。
// 系统键盘也是这样——点在两个键中间的缝里算离得近的那个键，不会什么都不发生（手机误触的一大来源）。

import SwiftUI

enum KeyboardLayout {
    /// 键区里键帽上方的留白（第一行的触摸范围一直到键区顶边）。
    static let topPadding: CGFloat = 8

    static func slots(layer: KeyLayer, showsGlobe: Bool, size: CGSize) -> [KeySlot] {
        let unit = KeyStyle.unit(for: size.width)
        var rows: [[(key: Key, x: CGFloat, width: CGFloat)]] = layer.rows.map { place($0, unit: unit, width: size.width) }
        rows.append(bottomRow(layer: layer, showsGlobe: showsGlobe, unit: unit, width: size.width))

        var slots: [KeySlot] = []
        for (index, row) in rows.enumerated() {
            let top = topPadding + CGFloat(index) * (KeyStyle.keyHeight + KeyStyle.rowSpacing)
            let bottom = top + KeyStyle.keyHeight
            let cellTop = index == 0 ? 0 : top - KeyStyle.rowSpacing / 2
            let cellBottom = index == rows.count - 1 ? size.height : bottom + KeyStyle.rowSpacing / 2
            for (column, placed) in row.enumerated() {
                let left = column == 0 ? 0 : (row[column - 1].x + row[column - 1].width + placed.x) / 2
                let right = column == row.count - 1
                    ? size.width
                    : (placed.x + placed.width + row[column + 1].x) / 2
                slots.append(KeySlot(
                    key: placed.key,
                    cell: CGRect(x: left, y: cellTop, width: right - left, height: cellBottom - cellTop),
                    insets: EdgeInsets(
                        top: top - cellTop,
                        leading: placed.x - left,
                        bottom: cellBottom - bottom,
                        trailing: right - placed.x - placed.width)))
            }
        }
        return slots
    }

    /// 上面三行：两端功能键贴边，中间字母键按字母键宽居中，或（数字层第三行）拉宽填满。
    private static func place(_ row: KeyRow, unit: CGFloat, width: CGFloat) -> [(key: Key, x: CGFloat, width: CGFloat)] {
        let gap = KeyStyle.keySpacing
        let edge = unit * KeyStyle.edgeKeyUnits
        var placed: [(key: Key, x: CGFloat, width: CGFloat)] = []
        var middleStart = KeyStyle.sideMargin
        var middleEnd = width - KeyStyle.sideMargin
        if let leading = row.leading {
            placed.append((leading, KeyStyle.sideMargin, edge))
            middleStart += edge + gap
        }
        if row.trailing != nil {
            middleEnd -= edge + gap
        }
        let count = CGFloat(row.middle.count)
        let keyWidth = row.stretchesMiddle ? (middleEnd - middleStart - gap * (count - 1)) / count : unit
        let total = keyWidth * count + gap * (count - 1)
        var x = row.stretchesMiddle ? middleStart : middleStart + (middleEnd - middleStart - total) / 2
        for key in row.middle {
            placed.append((key, x, keyWidth))
            x += keyWidth + gap
        }
        if let trailing = row.trailing {
            placed.append((trailing, width - KeyStyle.sideMargin - edge, edge))
        }
        return placed
    }

    /// 底行：切层、😀（或地球键）、空格吃掉剩余宽度、换行。
    private static func bottomRow(layer: KeyLayer, showsGlobe: Bool, unit: CGFloat, width: CGFloat) -> [(key: Key, x: CGFloat, width: CGFloat)] {
        let gap = KeyStyle.keySpacing
        let side = unit * KeyStyle.bottomSideUnits
        let returnWidth = unit * KeyStyle.returnUnits
        var x = KeyStyle.sideMargin
        let switchKey = Key.layer(layer == .letters ? .digits : .letters)
        let second = showsGlobe ? Key.globe : Key.emoji
        let spaceWidth = width - KeyStyle.sideMargin * 2 - side * 2 - returnWidth - gap * 3
        var placed: [(key: Key, x: CGFloat, width: CGFloat)] = []
        for (key, keyWidth) in [(switchKey, side), (second, side), (Key.space, spaceWidth), (Key.returnKey, returnWidth)] {
            placed.append((key, x, keyWidth))
            x += keyWidth + gap
        }
        return placed
    }
}
