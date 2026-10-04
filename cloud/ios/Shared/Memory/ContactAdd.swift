// 键盘里新建对象：名字怎么收拾、桥的回复怎么解（qj_memory_add_contact：成功 {"id"}，失败 {"code","message"}）。

import Foundation

enum ContactAdd {
    /// 键盘里起的名字最多这么多字（名字只是给自己看的称呼，App 里还能改）。
    static let maxNameChars = 12

    /// 去掉首尾空白、截到 12 字；空的返回 nil。
    static func name(_ draft: String) -> String? {
        let trimmed = draft.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return nil }
        return String(trimmed.prefix(maxNameChars))
    }

    /// 解桥的回复：有 id 是建好了，否则按失败解；什么都解不出来当 other。
    static func parse(_ json: String?) -> Result<String, MemoryFailure> {
        struct Added: Decodable {
            let id: String
        }
        if let data = json?.data(using: .utf8), let added = try? JSONDecoder().decode(Added.self, from: data) {
            return .success(added.id)
        }
        return .failure(MemoryFailure.decode(json) ?? MemoryFailure(code: .other, message: "没建成，请再试一次"))
    }

    /// 输入条空着时的占位文字。
    static let placeholder = "新对象叫什么（名字或代号）"
}
