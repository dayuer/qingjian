// 一个场景的一组人：App 首页按恋爱、日常、工作分组，键盘面板列当前场景的人；每组各自最多 8 个（与桥的上限一致）。

struct SceneGroup: Identifiable, Equatable {
    let scene: String

    let people: [MemoryContact]

    var id: String { scene }

    /// 每个场景各自最多几个人。
    static let limit = 8

    /// 三个场景都在（没人的组也在，好从那里「加一个人」），按 `MemoryScope.homeOrder` 排，组内保持名单里的顺序。
    static func all(_ contacts: [MemoryContact]) -> [SceneGroup] {
        MemoryScope.homeOrder.map { SceneGroup(scene: $0, people: people(in: $0, from: contacts)) }
    }

    static func people(in scene: String, from contacts: [MemoryContact]) -> [MemoryContact] {
        contacts.filter { $0.scene == scene }
    }

    var title: String { MemoryScope.title(of: scene) }

    /// 「2 / 8」。
    var countLabel: String { "\(people.count) / \(Self.limit)" }

    var isFull: Bool { people.count >= Self.limit }

    /// 满了时组下面的一句，与桥的 contact_limit 文案一致。
    var fullNote: String { MemoryFailure.contactLimit(scene: scene).message }

    /// 组标题「恋爱 · 2 / 8」。
    var header: String { "\(title) · \(countLabel)" }
}
