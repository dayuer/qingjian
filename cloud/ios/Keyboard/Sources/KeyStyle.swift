// 键盘的尺寸与颜色，量自 iOS 26 简体拼音键盘（iPhone 竖屏 390pt 宽）。

import SwiftUI
import UIKit

enum KeyStyle {
    static let keyHeight: CGFloat = 42

    static let rowSpacing: CGFloat = 12

    static let keySpacing: CGFloat = 6.5

    static let sideMargin: CGFloat = 6

    static let cornerRadius: CGFloat = 8.5

    static let candidateBarHeight: CGFloat = 48

    /// ⇧ / ⌫ / 切层键的宽度，以字母键宽为 1。
    static let edgeKeyUnits: CGFloat = 1.38

    /// 底行左边两个键（切层、😀）的宽度。
    static let bottomSideUnits: CGFloat = 1.3

    static let returnUnits: CGFloat = 2.85

    /// 键帽：浅色模式白、深色模式中灰；按下时变暗一档。
    static let keyFill = Color(UIColor { traits in
        traits.userInterfaceStyle == .dark ? UIColor(white: 0.42, alpha: 1) : .white
    })

    static let pressedFill = Color(UIColor { traits in
        traits.userInterfaceStyle == .dark ? UIColor(white: 0.3, alpha: 1) : UIColor(white: 0.82, alpha: 1)
    })

    /// 候选栏首选的灰底。
    static let highlightFill = Color(UIColor { traits in
        traits.userInterfaceStyle == .dark ? UIColor(white: 1, alpha: 0.18) : UIColor(white: 0, alpha: 0.07)
    })

    /// 字母键宽：一行 10 个键加两侧边距正好铺满。
    static func unit(for width: CGFloat) -> CGFloat {
        (width - sideMargin * 2 - keySpacing * 9) / 10
    }
}
