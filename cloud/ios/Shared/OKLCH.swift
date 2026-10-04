// oklch 颜色换算成 sRGB（Björn Ottosson 的 OKLab 矩阵，sRGB 传输函数），Theme 的色值用它核对。

import Foundation

enum OKLCH {
    /// 每个通道 0–255；超出 sRGB 色域的通道夹到边界。
    static func srgb(l: Double, c: Double, h: Double) -> (r: Int, g: Int, b: Int) {
        let radians = h * .pi / 180
        let a = c * cos(radians)
        let b = c * sin(radians)
        let l1 = pow(l + 0.3963377774 * a + 0.2158037573 * b, 3)
        let m1 = pow(l - 0.1055613458 * a - 0.0638541728 * b, 3)
        let s1 = pow(l - 0.0894841775 * a - 1.2914855480 * b, 3)
        let red = 4.0767416621 * l1 - 3.3077115913 * m1 + 0.2309699292 * s1
        let green = -1.2684380046 * l1 + 2.6097574011 * m1 - 0.3413193965 * s1
        let blue = -0.0041960863 * l1 - 0.7034186147 * m1 + 1.7076147010 * s1
        return (channel(red), channel(green), channel(blue))
    }

    private static func channel(_ linear: Double) -> Int {
        let clamped = min(max(linear, 0), 1)
        let encoded = clamped <= 0.0031308 ? 12.92 * clamped : 1.055 * pow(clamped, 1 / 2.4) - 0.055
        return Int((encoded * 255).rounded())
    }
}
