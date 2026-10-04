// 素笺的强调色：灰绿，只用在和对象有关的地方（对象牌子、提示行、对象卡与选择面板）。工作 / 日常场景完全不用。
// 设计稿 theme.css 的 oklch 值，已用 `OKLCH.srgb`（Tests/ThemeTests 核对）换算成 sRGB：
//                浅色                              深色
//   --accent      oklch(0.89  0.06  150)  #C0E7C6  oklch(0.42 0.06  150)  #34563B  圆点、头像底、选中描边
//   --accent-ink  oklch(0.38  0.05  150)  #2E4A34  oklch(0.86 0.06  150)  #B6DDBD  强调色上的文字、按钮文字
//   --accent-soft oklch(0.965 0.025 150)  #E8F9EB  oklch(0.26 0.02  150)  #1D271F  提示行与牌子的底色
// ink / ink-2 不自定义，跟系统的 label / secondaryLabel 走；云端候选、剪贴板、润色的标识用 ink-2，不用强调色。

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

    /// 正文与按钮文字：系统 label。
    static let ink = Color(UIColor.label)

    /// 次要文字与云端内容的标识：系统 secondaryLabel。
    static let ink2 = Color(UIColor.secondaryLabel)
}
