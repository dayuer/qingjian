// MemoryStore 读写记忆用的两个函数：正式走桥（MemoryFiles → qj_memory_read / qj_memory_write），
// 测试换成假的，好把锁超时、冲突、读不出这些失败路径逐条测到。

import Foundation

struct MemoryBackend {
    /// 整份读；读不出（锁屏、参数无效）为 nil。
    var read: (URL) -> MemorySnapshot?

    /// 整份写回；成功为 nil。
    var write: (MemorySnapshot, URL) -> MemoryFailure?

    static var bridge: MemoryBackend {
        MemoryBackend(
            read: { MemoryFiles.read(userDirectory: $0) },
            write: { MemoryFiles.write($0, userDirectory: $1) })
    }
}
