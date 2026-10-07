// 素笺的强调色：灰绿，只用在和对象有关的地方（对象牌子、提示行、对象卡）。
// 设计稿 theme.css 的 oklch 值，已用 `OKLCH.srgb`（Tests/ThemeTests 核对）换算成 sRGB：
//                浅色                              深色
//   --accent      oklch(0.89  0.06  150)  #C0E7C6  oklch(0.42 0.06  150)  #34563B  圆点、头像底、选中描边
//   --accent-ink  oklch(0.38  0.05  150)  #2E4A34  oklch(0.86 0.06  150)  #B6DDBD  强调色上的文字、按钮文字
//   --accent-soft oklch(0.965 0.025 150)  #E8F9EB  oklch(0.26 0.02  150)  #1D271F  提示行与牌子的底色
//   提示行下沿    oklch(0.91  0.035 150)  #D2E8D5  oklch(0.33 0.025 150)  #2C392F  .k-hint 的 1px 分隔线
// ink 跟系统 label 走；ink-2 照设计稿 oklch(0.42 0 0)（#4D4D4D，比 ink-3 深；系统 secondaryLabel 比 ink-3 还淡，层次会反过来），
// 深色取对称的 oklch(0.82 0 0)。云端候选、剪贴板、润色的标识用 ink-2，不用强调色。
// ink-3（设计稿 oklch(0.56 0 0)，#747474）是更淡的说明文字：「知道了」、页脚。

import SwiftUI
import UIKit

enum Theme {
    static let accent = ThemeColor(
        light: ThemeSwatch(hex: 0xC0E7C6, l: 0.89, c: 0.06, h: 150),
        dark: ThemeSwatch(hex: 0x34563B, l: 0.42, c: 0.06, h: 150))

    static let accentInk = ThemeColor(
        light: ThemeSwatch(hex: 0x2E4A34, l: 0.38, c: 0.05, h: 150),
        dark: ThemeSwatch(hex: 0xB6DDBD, l: 0.86, c: 0.06, h: 150))

    static let accentSoft = ThemeColor(
        light: ThemeSwatch(hex: 0xE8F9EB, l: 0.965, c: 0.025, h: 150),
        dark: ThemeSwatch(hex: 0x1D271F, l: 0.26, c: 0.02, h: 150))

    static let hintLine = ThemeColor(
        light: ThemeSwatch(hex: 0xD2E8D5, l: 0.91, c: 0.035, h: 150),
        dark: ThemeSwatch(hex: 0x2C392F, l: 0.33, c: 0.025, h: 150))

    /// 正文与按钮文字：系统 label。
    static let ink = Color(UIColor.label)

    /// 灰底小块（设计稿 --paper-2 oklch(0.965 0 0)，事件行尾的种类标签）。
    static let paper2 = Color(UIColor { traits in
        traits.userInterfaceStyle == .dark ? UIColor(white: 0.17, alpha: 1) : UIColor(white: 0xF3 / 255.0, alpha: 1)
    })

    static let ink2Tone = ThemeColor(
        light: ThemeSwatch(hex: 0x4D4D4D, l: 0.42, c: 0, h: 0),
        dark: ThemeSwatch(hex: 0xC4C4C4, l: 0.82, c: 0, h: 0))

    /// 次要文字与云端内容的标识（设计稿 --ink-2）。
    static let ink2 = ink2Tone.color

    /// 更淡的说明文字：浅色 #747474，深色取对称的浅灰。
    static let ink3 = Color(UIColor { traits in
        traits.userInterfaceStyle == .dark ? UIColor(white: 0.62, alpha: 1) : UIColor(white: 0x74 / 255.0, alpha: 1)
    })
}
