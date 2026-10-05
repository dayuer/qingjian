// 主 App 的字体入口：一律用小米 MiSans 可变字体（随包的 MiSansVF.ttf，Info.plist 的 UIAppFonts 注册）。键盘扩展不打包它，照旧用系统字体。
// `font(size:weight:)` 是定字号（与原来的 `.system(size:)` 一样不随动态字体缩放），`body` / `footnote` 这类语义字号随动态字体缩放。
// 取不到字体时回退到系统字体并记一次日志；构建时 check-app-fonts.sh 已保证包里有字体，回退只是防御。

import os
import SwiftUI
import UIKit

enum AppFont {
    /// SwiftUI `.custom` 用的名字（字体的 full name）。
    static let name = "MiSans VF"

    static let postScriptName = "MiSansVF"

    /// 字重轴 wght 的 OpenType 标签（'wght'）。
    static let weightAxis = 0x7767_6874

    static var isAvailable: Bool { UIFont(name: postScriptName, size: 17) != nil }

    static func font(size: CGFloat, weight: Font.Weight = .regular) -> Font {
        font(size: size, weight: weight, available: isAvailable)
    }

    static func font(size: CGFloat, weight: Font.Weight, available: Bool) -> Font {
        guard available else {
            logMissingOnce()
            return .system(size: size, weight: weight)
        }
        return .custom(name, fixedSize: size).weight(weight)
    }

    /// 随动态字体缩放的字号：`size` 是默认（Large）档的大小，按 `style` 的比例缩放。
    static func font(size: CGFloat, relativeTo style: Font.TextStyle, weight: Font.Weight = .regular) -> Font {
        font(size: size, relativeTo: style, weight: weight, available: isAvailable)
    }

    static func font(size: CGFloat, relativeTo style: Font.TextStyle, weight: Font.Weight, available: Bool) -> Font {
        guard available else {
            logMissingOnce()
            return .system(style).weight(weight)
        }
        return .custom(name, size: size, relativeTo: style).weight(weight)
    }

    // 语义字号：大小取 iOS 默认（Large）档的系统字号。
    static var largeTitle: Font { font(size: 34, relativeTo: .largeTitle) }
    static var title: Font { font(size: 28, relativeTo: .title) }
    static var title2: Font { font(size: 22, relativeTo: .title2) }
    static var title3: Font { font(size: 20, relativeTo: .title3) }
    static var headline: Font { font(size: 17, relativeTo: .headline, weight: .semibold) }
    static var body: Font { font(size: 17, relativeTo: .body) }
    static var callout: Font { font(size: 16, relativeTo: .callout) }
    static var subheadline: Font { font(size: 15, relativeTo: .subheadline) }
    static var footnote: Font { font(size: 13, relativeTo: .footnote) }
    static var caption: Font { font(size: 12, relativeTo: .caption) }
    static var caption2: Font { font(size: 11, relativeTo: .caption2) }

    /// 设计稿里占位说明用等宽字；MiSans 没有等宽，用系统等宽。
    static func mono(size: CGFloat) -> Font {
        .system(size: size, design: .monospaced)
    }

    /// UIKit 那一层（导航栏、标签栏）用：按字重轴取可变字体的实例；取不到时回退系统字体。
    static func uiFont(size: CGFloat, weight: UIFont.Weight = .regular) -> UIFont {
        let descriptor = UIFontDescriptor(fontAttributes: [
            .name: postScriptName,
            UIFontDescriptor.AttributeName(rawValue: kCTFontVariationAttribute as String): [weightAxis: axisValue(weight)],
        ])
        let font = UIFont(descriptor: descriptor, size: size)
        guard font.fontName.hasPrefix(postScriptName) else {
            logMissingOnce()
            return .systemFont(ofSize: size, weight: weight)
        }
        return font
    }

    /// UIFont.Weight 换成 wght 轴上的值（MiSans VF 的轴是 150–700）。
    static func axisValue(_ weight: UIFont.Weight) -> CGFloat {
        let value = weight.rawValue
        if value < UIFont.Weight.thin.rawValue { return 150 }
        if value < UIFont.Weight.light.rawValue { return 200 }
        if value < UIFont.Weight.regular.rawValue { return 300 }
        if value < UIFont.Weight.medium.rawValue { return 400 }
        if value < UIFont.Weight.semibold.rawValue { return 500 }
        if value < UIFont.Weight.bold.rawValue { return 600 }
        return 700
    }

    nonisolated(unsafe) private static var loggedMissing = false

    private static func logMissingOnce() {
        guard !loggedMissing else { return }
        loggedMissing = true
        Logger(subsystem: "sujian.synon.ai", category: "font")
            .error("包里取不到 \(postScriptName, privacy: .public)，App 字体回退到系统字体")
    }
}
