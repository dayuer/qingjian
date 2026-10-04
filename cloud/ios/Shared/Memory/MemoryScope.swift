// 键盘当前的场景与对象，以及各场景上次选的人（memory/state.json；qj_scope_get 给的就是这三项）。提示开关在各个对象上（MemoryContact）。
// 每个场景各有一组人（各自最多 8 个，互不相通）；恋爱、日常出提示与日子提醒，工作只做分区学习。

struct MemoryScope: Codable, Equatable, Sendable {
    static let daily = "daily"

    static let dating = "dating"

    static let work = "work"

    /// App 首页分组的顺序。
    static let homeOrder = [dating, daily, work]

    /// 键盘选择面板分段的顺序。
    static let pickerOrder = [daily, dating, work]

    var scene = MemoryScope.daily

    var contactId: String?

    /// 场景 → 上次选的人；切场景不指定人时桥按它回到那个人。
    var last: [String: String] = [:]

    enum CodingKeys: String, CodingKey {
        case scene, last
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

    /// 这个场景出不出提示行、对象卡与日子提醒（App 的今天、本周也按它）：恋爱、日常出，工作不出。
    static func reminds(_ scene: String) -> Bool { scene != work }
}

extension MemoryScope {
    /// 缺的字段按缺省（init 写在扩展里，成员逐一构造器才留得住）。
    init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        self.init()
        scene = try container.decodeIfPresent(String.self, forKey: .scene) ?? MemoryScope.daily
        contactId = try container.decodeIfPresent(String.self, forKey: .contactId)
        last = try container.decodeIfPresent([String: String].self, forKey: .last) ?? [:]
    }
}
