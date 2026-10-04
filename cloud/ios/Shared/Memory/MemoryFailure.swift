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

    /// 给用户看的话：锁超时用固定文案，其余照桥给的（人数上限的话带场景名，如「日常最多 8 个人」；桥没给时用通用的）。
    var userMessage: String {
        switch code {
        case .contactLimit: message.isEmpty ? "每个场景最多 \(SceneGroup.limit) 个人" : message
        case .lockTimeout: "键盘正在写记忆，请稍后再试"
        default: message
        }
    }

    /// Swift 侧先挡住的人数上限，文案与桥的一致。
    static func contactLimit(scene: String) -> MemoryFailure {
        MemoryFailure(code: .contactLimit, message: "\(MemoryScope.title(of: scene))最多 \(SceneGroup.limit) 个人")
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
