// 提示行的一条：qj_memory_hint 的 JSON。

struct MemoryHint: Decodable, Equatable, Sendable {
    let cardId: String

    let text: String

    let reason: Reason

    /// 对象还有别的卡（「展开」看得到）。
    let more: Bool

    enum Reason: String, Decodable, Sendable {
        case match, today
    }

    enum CodingKeys: String, CodingKey {
        case text, reason, more
        case cardId = "card_id"
    }
}
