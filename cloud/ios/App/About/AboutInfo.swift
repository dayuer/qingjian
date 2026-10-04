// 关于页的文案与链接：版本号取 Info.plist 的 CFBundleShortVersionString；GPL 署名是许可证要求，不要删。

import Foundation

enum AboutInfo {
    static let name = "素笺"

    /// 许可证要求保留的署名。
    static let attribution = "基于开源的青简输入法（GPL-3.0）"

    static let sourceURL = URL(string: "https://github.com/dayuer/qingjian")!

    static let licenseURL = URL(string: "https://www.gnu.org/licenses/gpl-3.0.html")!

    /// 缺版本号时只显示名字，不显示空的「版本 」。
    static func versionText(info: [String: Any]?) -> String {
        guard let version = info?["CFBundleShortVersionString"] as? String, !version.isEmpty else { return "" }
        return "版本 \(version)"
    }
}
