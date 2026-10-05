// 桥写记忆失败时返回的 JSON：{"code": "...", "message": "..."}。code 给界面分支，message 是给用户看的中文。
// material_limit 另带 remaining（还剩几个空位）与 needed（这次要几条），记一笔条按它们写原因。

import Foundation

struct MemoryFailure: Error, Equatable, Sendable {
    let code: Code

    let message: String

    /// material_limit 时这个人还剩几个空位；别的 code 没有。
    var remaining: Int?

    /// material_limit 时这次切出了几条。
    var needed: Int?

    /// 与桥的 `MemoryError::code` 一一对应；认不得的值按 other。
    enum Code: String, Decodable, Sendable {
        case pinLimit = "pin_limit"
        case invalid
        case conflict
        case materialLimit = "material_limit"
        case lockTimeout = "lock_timeout"
        case io
        case other

        init(from decoder: any Decoder) throws {
            let raw = try decoder.singleValueContainer().decode(String.self)
            self = Code(rawValue: raw) ?? .other
        }
    }

    /// 给用户看的话：锁超时与素材满了用固定文案，其余照桥给的（置顶超了的话桥会带「一个场景最多置顶 4 个人」）。
    /// 素材满了按 remaining / needed 写（NoteBarText.materialLimit）；旧桥没带时按没有空位写。
    var userMessage: String {
        switch code {
        case .pinLimit: message.isEmpty ? "一个场景最多置顶 \(MemoryScene.maxPinned) 个人" : message
        case .materialLimit: NoteBarText.materialLimit(remaining: remaining ?? 0, needed: needed ?? 1)
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
            let remaining: Int?
            let needed: Int?
        }
        if let data = json.data(using: .utf8), let wire = try? JSONDecoder().decode(Wire.self, from: data) {
            return MemoryFailure(code: wire.code, message: wire.message, remaining: wire.remaining, needed: wire.needed)
        }
        return MemoryFailure(code: .other, message: json)
    }
}
