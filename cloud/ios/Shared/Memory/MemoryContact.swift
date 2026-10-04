// 记忆里的一个人（contacts.json 的一项）。名字只在这里，对象目录名用随机 id。两个提示开关按人设置，旧文件没有时按开。

import Foundation

struct MemoryContact: Codable, Identifiable, Hashable, Sendable {
    let id: String

    var name: String

    var pronoun: MemoryPronoun

    var scene: String

    let createdAt: Int64

    /// 打字时按这个人的卡片提示。
    var hintOn = true

    /// 这个人的日子与约定快到时提醒。
    var remindOn = true

    enum CodingKeys: String, CodingKey {
        case id, name, pronoun, scene
        case createdAt = "created_at"
        case hintOn = "hint_on"
        case remindOn = "remind_on"
    }

    /// 恋爱场景的新对象。
    static func new(name: String, pronoun: MemoryPronoun) -> MemoryContact {
        MemoryContact(
            id: MemoryID.make(), name: name, pronoun: pronoun, scene: MemoryScope.dating,
            createdAt: Int64(Date().timeIntervalSince1970))
    }

    /// 认识了几天：按北京时间的日历日，建的那天算第 1 天。
    func knownDays(now: Date = Date()) -> Int {
        MemoryDate.daysBetween(Date(timeIntervalSince1970: TimeInterval(createdAt)), now) + 1
    }
}

extension MemoryContact {
    /// 缺 `hint_on` / `remind_on` 的旧文件按开（init 写在扩展里，成员逐一构造器才留得住）。
    init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        self.init(
            id: try container.decode(String.self, forKey: .id),
            name: try container.decode(String.self, forKey: .name),
            pronoun: try container.decodeIfPresent(MemoryPronoun.self, forKey: .pronoun) ?? .ta,
            scene: try container.decode(String.self, forKey: .scene),
            createdAt: try container.decode(Int64.self, forKey: .createdAt),
            hintOn: try container.decodeIfPresent(Bool.self, forKey: .hintOn) ?? true,
            remindOn: try container.decodeIfPresent(Bool.self, forKey: .remindOn) ?? true)
    }
}
