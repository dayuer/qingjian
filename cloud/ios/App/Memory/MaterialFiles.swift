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

    /// **还没归到人的**素材（首页「+ 记一条」记下的、还没补上归给谁）；读不出为 nil。
    /// 与按对象的那份同形，所以直接复用 `MaterialList`。
    static func unassignedList(userDirectory: URL) -> MaterialList? {
        let raw = userDirectory.path.withCString { qj_memory_unassigned_materials($0) }
        return MemoryFiles.decode(MemoryFiles.take(raw))
    }

    /// 首页「+ 记一条」：存一条不绑对象的素材；成功为 nil。
    /// 失败可能是 `material_limit`（无主素材满了），文案由 `MaterialDisplay` 那边给。
    static func addUnassigned(
        userDirectory: URL, text: String, source: MemoryMaterial.Source
    ) -> MemoryFailure? {
        let raw = userDirectory.path.withCString { dir in
            text.withCString { body in
                source.rawValue.withCString { qj_memory_unassigned_note(dir, body, $0) }
            }
        }
        return MemoryFailure.decode(MemoryFiles.take(raw))
    }

    /// 「补上」：把无主素材里的一条归到某个人名下；成功为 nil。
    static func assign(userDirectory: URL, clientId: String, contactId: String) -> MemoryFailure? {
        let raw = userDirectory.path.withCString { dir in
            clientId.withCString { c in contactId.withCString { qj_memory_assign_material(dir, c, $0) } }
        }
        return MemoryFailure.decode(MemoryFiles.take(raw))
    }
}
