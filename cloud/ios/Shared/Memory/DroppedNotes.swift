// 桥 qj_memory_dropped 的返回：拿不到锁排队的记一笔，补写时被拒绝的条数（只有条数与原因，没有原文）。取一次桥就清零。

struct DroppedNotes: Decodable, Equatable, Sendable {
    /// 这个人的待整理满了。
    var materialLimit = 0

    /// 这个人已经被忘掉。
    var contactGone = 0

    enum CodingKeys: String, CodingKey {
        case materialLimit = "material_limit"
        case contactGone = "contact_gone"
    }

    init(materialLimit: Int = 0, contactGone: Int = 0) {
        self.materialLimit = materialLimit
        self.contactGone = contactGone
    }

    init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        materialLimit = try container.decodeIfPresent(Int.self, forKey: .materialLimit) ?? 0
        contactGone = try container.decodeIfPresent(Int.self, forKey: .contactGone) ?? 0
    }
}
