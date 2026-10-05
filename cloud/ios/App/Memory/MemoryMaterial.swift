// 一条待整理素材：「记一笔」存下的原话（桥 qj_memory_materials 里 materials 的一项）。原话原样，名字和时间行都在里面。

import Foundation

struct MemoryMaterial: Decodable, Equatable, Identifiable, Sendable {
    let clientId: String

    /// 原话，原样显示，不删改。
    let text: String

    /// 记下的时刻（Unix 秒）。
    let at: Int64

    let source: Source

    var id: String { clientId }

    enum Source: String, Decodable, Sendable {
        case clipboard
        case typed

        /// 认不得的按手写（与桥的 MaterialSource 一致）。
        init(from decoder: any Decoder) throws {
            let raw = try decoder.singleValueContainer().decode(String.self)
            self = Source(rawValue: raw) ?? .typed
        }
    }

    enum CodingKeys: String, CodingKey {
        case clientId = "client_id"
        case text
        case at
        case source
    }
}
