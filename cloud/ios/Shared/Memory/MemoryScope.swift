// 键盘当前的场景与对象（memory/state.json；qj_scope_get 给的就是这两项）。提示开关在各个对象上（MemoryContact）。

struct MemoryScope: Codable, Equatable, Sendable {
    static let daily = "daily"

    static let dating = "dating"

    static let work = "work"

    var scene = MemoryScope.daily

    var contactId: String?

    enum CodingKeys: String, CodingKey {
        case scene
        case contactId = "contact_id"
    }

    /// 场景的中文名。
    static func title(of scene: String) -> String {
        switch scene {
        case dating: "恋爱"
        case work: "工作"
        default: "日常"
        }
    }
}

extension MemoryScope {
    /// 缺的字段按缺省（init 写在扩展里，成员逐一构造器才留得住）。
    init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        self.init()
        scene = try container.decodeIfPresent(String.self, forKey: .scene) ?? MemoryScope.daily
        contactId = try container.decodeIfPresent(String.self, forKey: .contactId)
    }
}
