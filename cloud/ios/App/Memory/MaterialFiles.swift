// 经桥读、删一个人的待整理素材（qj_memory_materials / qj_memory_material_delete），App 用；都要等 memory/.lock，只在 MaterialsWorker 的后台队列上调。

import Foundation
import QingjianBridge

enum MaterialFiles {
    /// 没整理的素材；参数无效或读不了（开机后还没解锁过、等锁超时）时为 nil。
    static func list(userDirectory: URL, contactId: String) -> MaterialList? {
        let raw = userDirectory.path.withCString { dir in contactId.withCString { qj_memory_materials(dir, $0) } }
        return MemoryFiles.decode(MemoryFiles.take(raw))
    }

    /// 删一条；成功（含本来就没有）返回 nil。
    static func delete(userDirectory: URL, contactId: String, clientId: String) -> MemoryFailure? {
        let raw = userDirectory.path.withCString { dir in
            contactId.withCString { c in clientId.withCString { qj_memory_material_delete(dir, c, $0) } }
        }
        return MemoryFailure.decode(MemoryFiles.take(raw))
    }
}
