// 设计稿里三种细线的灰（都是不透明的中性灰，比系统 separator 淡），深色取对称的深灰：
//   line  `--line` oklch(0.89 0 0)：工具栏下沿、线框按钮、App 里的分隔线
//   ring  `.group` 外圈 oklch(0.92 0 0)：白底卡片组的那一圈
//   row   `.row` 行间 oklch(0.94 0 0)：卡片组里行与行之间

import SwiftUI
import UIKit

enum Hairline {
    static let line = gray(light: 0xDB, dark: 0.24)

    static let ring = gray(light: 0xE2, dark: 0.28)

    static let row = gray(light: 0xEA, dark: 0.25)

    private static func gray(light: Int, dark: CGFloat) -> Color {
        Color(UIColor { traits in
            traits.userInterfaceStyle == .dark ? UIColor(white: dark, alpha: 1) : UIColor(white: CGFloat(light) / 255, alpha: 1)
        })
    }
}
