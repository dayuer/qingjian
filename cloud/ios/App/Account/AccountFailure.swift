// 桥的账号操作失败时返回的 JSON：`{"code": "...", "message": "..."}`。message 是给用户看的中文，原样显示；code 给界面分支用。

import Foundation

struct AccountFailure: Equatable {
    let code: Code

    let message: String

    /// 与桥的 `failure.rs` 一一对应；认不得的值按 other 处理。
    enum Code: String, Codable, Equatable {
        case authFailed = "auth_failed"
        case lockedToday = "locked_today"
        case unauthorized
        case notConfigured = "not_configured"
        case rateLimited = "rate_limited"
        case forbidden
        case unreachable
        case invalidArgument = "invalid_argument"
        case notSignedIn = "not_signed_in"
        case other

        init(from decoder: any Decoder) throws {
            let raw = try decoder.singleValueContainer().decode(String.self)
            self = Code(rawValue: raw) ?? .other
        }
    }

    /// nil 表示成功。解析不了（如旧版桥返回的纯文案）时整段当 message、code 为 other。
    static func decode(_ json: String?) -> AccountFailure? {
        guard let json else { return nil }
        struct Wire: Decodable {
            let code: Code
            let message: String
        }
        if let data = json.data(using: .utf8),
           let wire = try? JSONDecoder().decode(Wire.self, from: data) {
            return AccountFailure(code: wire.code, message: wire.message)
        }
        return AccountFailure(code: .other, message: json)
    }
}
