// 账号下的一台已登录设备（会话），时间是 Unix 毫秒。

import Foundation

struct AccountDevice: Codable, Equatable, Identifiable {
    var id: Int64

    var name: String

    /// ios / macos / web。
    var platform: String

    var createdAt: Int64

    var lastSeen: Int64?

    /// 是不是这台 iPhone。
    var current: Bool

    /// 「iOS · 3分钟前活跃」。
    var detail: String {
        let platformTitle = switch platform {
        case "ios": "iOS"
        case "macos": "Mac"
        case "web": "网页"
        default: platform
        }
        let formatter = RelativeDateTimeFormatter()
        formatter.locale = Locale(identifier: "zh_CN")
        if let lastSeen {
            let at = Date(timeIntervalSince1970: Double(lastSeen) / 1000)
            return "\(platformTitle) · \(formatter.localizedString(for: at, relativeTo: .now))活跃"
        }
        let at = Date(timeIntervalSince1970: Double(createdAt) / 1000)
        return "\(platformTitle) · \(formatter.localizedString(for: at, relativeTo: .now))登录"
    }
}
