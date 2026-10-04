// 素笺的强调色：灰绿，只用在和对象有关的地方（对象牌子、提示行、对象卡与选择面板）。工作 / 日常场景完全不用。
// 设计稿 theme.css 的 oklch 值，已用 `OKLCH.srgb`（Tests/ThemeTests 核对）换算成 sRGB：
//   --accent      oklch(0.89  0.06  150)  #C0E7C6  圆点、头像底、选中描边
//   --accent-ink  oklch(0.38  0.05  150)  #2E4A34  强调色上的文字、按钮文字
//   --accent-soft oklch(0.965 0.025 150)  #E8F9EB  提示行与牌子的底色

import SwiftUI

enum Theme {
    static let accent = ThemeSwatch(hex: 0xC0E7C6, l: 0.89, c: 0.06, h: 150)

    static let accentInk = ThemeSwatch(hex: 0x2E4A34, l: 0.38, c: 0.05, h: 150)

    static let accentSoft = ThemeSwatch(hex: 0xE8F9EB, l: 0.965, c: 0.025, h: 150)
}
