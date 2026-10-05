// 键盘当前的场景与对象、各场景上次选的人、各人上次被选中的时间（memory/state.json；qj_scope_get 给的就是这几项）。
// 提示开关在各个对象上（MemoryContact）；场景本身（名字、有几个）在 MemoryScene，这个类型只管键盘的状态。

struct MemoryScope: Codable, Equatable, Sendable {
    /// 场景 id；认不得时按第一个场景走。
    var scene = ""

    var contactId: String?

    /// 场景 id → 上次选的人；切场景不指定人时桥按它回到那个人。
    var last: [String: String] = [:]

    /// 对象 id → 上次在键盘里选中的 Unix 秒（选择面板格子的「今天 / 3 天前」，也拿来排沟通情况）。
    var used: [String: Int64] = [:]

    enum CodingKeys: String, CodingKey {
        case scene, last, used
        case contactId = "contact_id"
    }
}

extension MemoryScope {
    /// 缺的字段按缺省（init 写在扩展里，成员逐一构造器才留得住）。
    init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        self.init()
        scene = try container.decodeIfPresent(String.self, forKey: .scene) ?? ""
        contactId = try container.decodeIfPresent(String.self, forKey: .contactId)
        last = try container.decodeIfPresent([String: String].self, forKey: .last) ?? [:]
        used = try container.decodeIfPresent([String: Int64].self, forKey: .used) ?? [:]
    }
}
