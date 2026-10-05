// 桥的 `qj_*` 操作用得上的失败 JSON：`{"code": "...", "message": "..."}`。message 是给用户看的中文，原样显示；code 给界面分支用。
// 账号与空间共用这一份（记忆那套走 `qj_memory_*`，另有自己的形状）。

import Foundation

struct BridgeFailure: Error, Equatable {
    let code: Code

    let message: String

    /// 与桥的 `failure.rs` 一一对应；认不得的值按 other 处理。
    enum Code: String, Codable, Equatable, CaseIterable {
        case authFailed = "auth_failed"
        case lockedToday = "locked_today"
        case unauthorized
        case notConfigured = "not_configured"
        case rateLimited = "rate_limited"
        case forbidden
        case unreachable
        case invalidArgument = "invalid_argument"
        case notSignedIn = "not_signed_in"
        case consentRequired = "consent_required"
        case badCode = "bad_code"
        case deviceLimit = "device_limit"
        case other

        init(from decoder: any Decoder) throws {
            let raw = try decoder.singleValueContainer().decode(String.self)
            self = Code(rawValue: raw) ?? .other
        }
    }

    /// nil 表示成功。解析不了（如旧版桥返回的纯文案）时整段当 message、code 为 other。
    static func decode(_ json: String?) -> BridgeFailure? {
        guard let json else { return nil }
        struct Wire: Decodable {
            let code: Code
            let message: String
        }
        if let data = json.data(using: .utf8),
           let wire = try? JSONDecoder().decode(Wire.self, from: data) {
            return BridgeFailure(code: wire.code, message: wire.message)
        }
        return BridgeFailure(code: .other, message: json)
    }
}
