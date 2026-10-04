// 桥写记忆失败时返回的 JSON：{"code": "...", "message": "..."}。code 给界面分支，message 是给用户看的中文。

import Foundation

struct MemoryFailure: Error, Equatable, Sendable {
    let code: Code

    let message: String

    /// 与桥的 `MemoryError::code` 一一对应；认不得的值按 other。
    enum Code: String, Decodable, Sendable {
        case contactLimit = "contact_limit"
        case invalid
        case conflict
        case lockTimeout = "lock_timeout"
        case io
        case other

        init(from decoder: any Decoder) throws {
            let raw = try decoder.singleValueContainer().decode(String.self)
            self = Code(rawValue: raw) ?? .other
        }
    }

    /// 给用户看的话：人数上限与锁超时用固定文案，其余照桥给的。
    var userMessage: String {
        switch code {
        case .contactLimit: "恋爱场景最多 8 个人"
        case .lockTimeout: "键盘正在写记忆，请稍后再试"
        default: message
        }
    }

    /// nil 表示成功；解析不了时整段当 message、code 为 other。
    static func decode(_ json: String?) -> MemoryFailure? {
        guard let json else { return nil }
        struct Wire: Decodable {
            let code: Code
            let message: String
        }
        if let data = json.data(using: .utf8), let wire = try? JSONDecoder().decode(Wire.self, from: data) {
            return MemoryFailure(code: wire.code, message: wire.message)
        }
        return MemoryFailure(code: .other, message: json)
    }
}
