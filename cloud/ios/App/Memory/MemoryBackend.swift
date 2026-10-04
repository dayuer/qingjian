// MemoryStore 读写记忆用的两个函数：正式走桥（MemoryFiles → qj_memory_read / qj_memory_write），
// 测试换成假的，好把锁超时、冲突、读不出这些失败路径逐条测到。在 MemoryWorker 的后台队列上调用，所以要 Sendable。

import Foundation

struct MemoryBackend: Sendable {
    /// 整份读；读不出（开机后还没解锁过、参数无效）为 nil。
    var read: @Sendable (URL) -> MemorySnapshot?

    /// 整份写回；成功为 nil。
    var write: @Sendable (MemorySnapshot, URL) -> MemoryFailure?

    static var bridge: MemoryBackend {
        MemoryBackend(
            read: { MemoryFiles.read(userDirectory: $0) },
            write: { MemoryFiles.write($0, userDirectory: $1) })
    }
}
