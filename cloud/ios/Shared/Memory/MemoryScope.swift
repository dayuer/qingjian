// 键盘当前的会话状态与各人上次被选中的时间（memory/state.json；qj_scope_get 给的就是这两项）。
// 提示开关在各个对象上（MemoryContact）。

struct MemoryScope: Codable, Equatable, Sendable {
    var contactId: String?

    /// 对象 id → 上次在键盘里选中的 Unix 秒（列人时按沟通情况排用）。
    var used: [String: Int64] = [:]

    enum CodingKeys: String, CodingKey {
        case used
        case contactId = "contact_id"
    }
}

extension MemoryScope {
    /// 缺的字段按缺省（init 写在扩展里，成员逐一构造器才留得住）。
    init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        self.init()
        contactId = try container.decodeIfPresent(String.self, forKey: .contactId)
        used = try container.decodeIfPresent([String: Int64].self, forKey: .used) ?? [:]
    }
}
