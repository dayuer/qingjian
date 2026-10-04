// 主题里随浅 / 深色模式切换的颜色：一对 ThemeSwatch，动态颜色按 userInterfaceStyle 取其一。

import SwiftUI
import UIKit

struct ThemeColor: Sendable {
    let light: ThemeSwatch

    let dark: ThemeSwatch

    var color: Color { Color(uiColor) }

    var uiColor: UIColor {
        let light = light.uiColor
        let dark = dark.uiColor
        return UIColor { traits in traits.userInterfaceStyle == .dark ? dark : light }
    }
}
