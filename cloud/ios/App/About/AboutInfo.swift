// 关于页的文案与链接：版本号取 Info.plist 的 CFBundleShortVersionString；GPL 署名与 MiSans 署名都是许可证要求，不要删。

import Foundation

enum AboutInfo {
    static let name = "素笺"

    /// 许可证要求保留的署名。
    static let attribution = "基于开源的青简输入法（GPL-3.0）"

    /// MiSans 许可第 2 条第 1 款：应在软件中特别注明使用了 MiSans 字体。
    static let fontAttribution = "本应用使用了 MiSans 字体（© 北京小米移动软件有限公司）"

    /// 随包的 MiSans 协议全文（第 2 条第 4 款要求随字体副本保留）。
    static let fontLicenseResource = "MiSans-LICENSE"

    static let sourceURL = URL(string: "https://github.com/dayuer/qingjian")!

    static let licenseURL = URL(string: "https://www.gnu.org/licenses/gpl-3.0.html")!

    static func fontLicenseText(bundle: Bundle = .main) -> String? {
        guard let url = bundle.url(forResource: fontLicenseResource, withExtension: "txt") else { return nil }
        return try? String(contentsOf: url, encoding: .utf8)
    }

    /// 缺版本号时只显示名字，不显示空的「版本 」。
    static func versionText(info: [String: Any]?) -> String {
        guard let version = info?["CFBundleShortVersionString"] as? String, !version.isEmpty else { return "" }
        return "版本 \(version)"
    }
}
