// 衬线标题字：照设计稿用宋体（系统自带的 Songti SC，不打包字体）。`.system(design: .serif)` 是 New York，没有中文字形，
// 中文会回落到苹方，所以要按 PostScript 名取。取不到（系统没带）时回退到 .serif 并记一次日志。

import os
import SwiftUI
import UIKit

enum SerifFont {
    /// 600 及以上用 Bold（宋体没有 Semibold），其余用 Regular。
    static func postScriptName(for weight: Font.Weight) -> String {
        switch weight {
        case .semibold, .bold, .heavy, .black: "STSongti-SC-Bold"
        default: "STSongti-SC-Regular"
        }
    }

    static func font(size: CGFloat, weight: Font.Weight = .semibold) -> Font {
        let name = postScriptName(for: weight)
        guard UIFont(name: name, size: size) != nil else {
            logMissingOnce(name)
            return .system(size: size, weight: weight, design: .serif)
        }
        return .custom(name, fixedSize: size)
    }

    nonisolated(unsafe) private static var loggedMissing = false

    private static func logMissingOnce(_ name: String) {
        guard !loggedMissing else { return }
        loggedMissing = true
        Logger(subsystem: "sujian.synon.ai", category: "font")
            .notice("系统没有宋体 \(name, privacy: .public)，衬线标题回退到 .serif")
    }
}
