// 键盘的尺寸与颜色，量自 iOS 26 简体拼音键盘（iPhone 15 Pro Max 竖屏 430pt 宽的截图）；键宽按屏宽算，其余是固定值。

import SwiftUI
import UIKit

enum KeyStyle {
    static let keyHeight: CGFloat = 44.5

    static let rowSpacing: CGFloat = 11.5

    static let keySpacing: CGFloat = 6.3

    static let sideMargin: CGFloat = 4

    static let cornerRadius: CGFloat = 8.5

    static let candidateBarHeight: CGFloat = 50

    /// 候选栏上方记忆提示行的高度。
    static let hintRowHeight: CGFloat = 34

    /// ⇧ / ⌫ / 切层键的宽度，以字母键宽为 1。
    static let edgeKeyUnits: CGFloat = 1.36

    /// 底行左边两个键（切层、😀）的宽度。
    static let bottomSideUnits: CGFloat = 1.31

    static let returnUnits: CGFloat = 2.78

    /// 键帽：浅色模式白、深色模式中灰；按下时变暗一档。
    static let keyFill = Color(UIColor { traits in
        traits.userInterfaceStyle == .dark ? UIColor(white: 0.42, alpha: 1) : .white
    })

    static let pressedFill = Color(UIColor { traits in
        traits.userInterfaceStyle == .dark ? UIColor(white: 0.3, alpha: 1) : UIColor(white: 0.82, alpha: 1)
    })

    /// 字母键宽：一行 10 个键加两侧边距正好铺满。
    static func unit(for width: CGFloat) -> CGFloat {
        (width - sideMargin * 2 - keySpacing * 9) / 10
    }
}
