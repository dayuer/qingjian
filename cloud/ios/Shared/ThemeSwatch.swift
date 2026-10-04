// 主题里的一个颜色：写死的 sRGB 十六进制，加上它出自的 oklch 值（单测用 OKLCH.srgb 核对两者一致）。

import SwiftUI
import UIKit

struct ThemeSwatch: Sendable {
    let hex: UInt32

    /// 出处：oklch 的明度、色度、色相（度）。
    let l: Double

    let c: Double

    let h: Double

    var r: Int { Int(hex >> 16) & 0xFF }

    var g: Int { Int(hex >> 8) & 0xFF }

    var b: Int { Int(hex) & 0xFF }

    var color: Color { Color(uiColor) }

    var uiColor: UIColor {
        UIColor(red: CGFloat(r) / 255, green: CGFloat(g) / 255, blue: CGFloat(b) / 255, alpha: 1)
    }
}
