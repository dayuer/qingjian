// 用户自己建、自己起名的分组（桥的 `memory/scenes.json` 一项）。id 建好不变——改名不动任何文件；
// 名字随用户改，场景之间没有行为差异。列表里第一个是默认场景：删掉别的场景时，里面的人挪到它。

import Foundation

struct MemoryScene: Codable, Identifiable, Hashable, Sendable {
    let id: String

    var name: String

    let createdAt: Int64

    enum CodingKeys: String, CodingKey {
        case id, name
        case createdAt = "created_at"
    }

    /// 场景名最多几个字（与桥的 `MAX_SCENE_NAME_CHARS` 一致）。
    static let maxNameChars = 8

    /// 一个场景里最多几个置顶（与桥的 `MAX_PINNED` 一致）。
    static let maxPinned = 4

    /// 新建一个场景；id 用与对象同一套的随机 32 位十六进制。
    static func new(name: String) -> MemoryScene {
        MemoryScene(
            id: MemoryID.make(), name: name,
            createdAt: Int64(Date().timeIntervalSince1970))
    }
}
