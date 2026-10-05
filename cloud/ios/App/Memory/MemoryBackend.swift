// MemoryStore 读写记忆用的两个函数：正式走桥（MemoryFiles → qj_memory_read / qj_memory_write），
// 测试换成假的，好把锁超时、冲突、读不出这些失败路径逐条测到。在 MemoryWorker 的后台队列上调用，所以要 Sendable。

import Foundation

struct MemoryBackend: Sendable {
    /// 整份读；读不出（开机后还没解锁过、参数无效）为 nil。
    var read: @Sendable (URL) -> MemorySnapshot?

    /// 整份写回；成功为 nil。
    var write: @Sendable (MemorySnapshot, URL) -> MemoryFailure?

    /// 加一个场景或改名（只写场景，不校验卡片、不比修订号）；成功为 nil。
    var putScene: @Sendable (URL, MemoryScene) -> MemoryFailure? = { _, _ in nil }

    /// 删一个场景，人挪到默认场景；成功为 nil。
    var deleteScene: @Sendable (URL, String) -> MemoryFailure? = { _, _ in nil }

    /// 首页「+ 记一条」的**无主素材**（不绑对象的那一桶）；读不出为 nil（当作空）。
    var unassigned: @Sendable (URL) -> [MemoryMaterial]? = { _ in [] }

    /// 往无主桶里记一条；成功为 nil。
    var addUnassigned: @Sendable (URL, String, MemoryMaterial.Source) -> MemoryFailure? = { _, _, _ in nil }

    /// 把无主桶里的一条归到某个人；成功为 nil。
    var assign: @Sendable (URL, String, String) -> MemoryFailure? = { _, _, _ in nil }

    static var bridge: MemoryBackend {
        MemoryBackend(
            read: { MemoryFiles.read(userDirectory: $0) },
            write: { MemoryFiles.write($0, userDirectory: $1) },
            putScene: { MemoryFiles.putScene($1, userDirectory: $0) },
            deleteScene: { MemoryFiles.deleteScene(id: $1, userDirectory: $0) },
            unassigned: { MaterialFiles.unassignedList(userDirectory: $0)?.materials },
            addUnassigned: { MaterialFiles.addUnassigned(userDirectory: $0, text: $1, source: $2) },
            assign: { MaterialFiles.assign(userDirectory: $0, clientId: $1, contactId: $2) })
    }
}
