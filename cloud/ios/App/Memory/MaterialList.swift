// 桥 qj_memory_materials 的返回：一个人没整理的素材（新的在上）与条数。「待整理 · n 条」和 180 条的提示都按 unprocessedCount。

struct MaterialList: Decodable, Equatable, Sendable {
    let unprocessedCount: Int

    let materials: [MemoryMaterial]

    enum CodingKeys: String, CodingKey {
        case unprocessedCount = "unprocessed_count"
        case materials
    }
}
