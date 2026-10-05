// App 整份读写的记忆数据：{"contacts","cards","revs","state","broken","unassigned"}，与桥的 MemorySnapshot 一一对应。
// revs 是读时各对象卡片的修订号，原样带回去写；桥据此发现键盘这期间「记一笔」改过（返回 conflict）。

struct MemorySnapshot: Codable, Equatable, Sendable {
    var contacts: [MemoryContact] = []

    /// 对象 id → 卡片。
    var cards: [String: [MemoryCard]] = [:]

    /// 对象 id → 读时的修订号。
    var revs: [String: UInt64] = [:]

    /// 键盘当前的场景与对象，只给显示；写回时桥不看。
    var state = MemoryScope()

    /// 这次读时卡片文件坏了、已备份的对象；写回时桥不看。
    var broken: [String] = []

    /// 「还没归到人的」卡片：首页「+ 记一条」先快速记下，事后「补上」归到某个人。
    var unassigned: [MemoryCard] = []

    enum CodingKeys: String, CodingKey {
        case contacts, cards, revs, state, broken, unassigned
    }
}

extension MemorySnapshot {
    init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        self.init()
        contacts = try container.decodeIfPresent([MemoryContact].self, forKey: .contacts) ?? []
        cards = try container.decodeIfPresent([String: [MemoryCard]].self, forKey: .cards) ?? [:]
        revs = try container.decodeIfPresent([String: UInt64].self, forKey: .revs) ?? [:]
        state = try container.decodeIfPresent(MemoryScope.self, forKey: .state) ?? MemoryScope()
        broken = try container.decodeIfPresent([String].self, forKey: .broken) ?? []
        unassigned = try container.decodeIfPresent([MemoryCard].self, forKey: .unassigned) ?? []
    }
}
