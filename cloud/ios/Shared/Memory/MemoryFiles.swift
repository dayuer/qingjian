// 经桥整份读写 memory/（qj_memory_read / qj_memory_write），场景的增删改名另走 qj_memory_put_scene / qj_memory_delete_scene。App 与键盘共用；键盘只读名单，写只在 App 里。

import Foundation
import QingjianBridge

enum MemoryFiles {
    /// 读 `userDirectory/memory/` 的全部数据；参数无效或解析不了时为 nil。
    static func read(userDirectory: URL) -> MemorySnapshot? {
        decode(take(userDirectory.path.withCString { qj_memory_read($0) }))
    }

    /// 整份写回；成功返回 nil。
    static func write(_ snapshot: MemorySnapshot, userDirectory: URL) -> MemoryFailure? {
        guard let data = try? JSONEncoder().encode(snapshot),
              let json = String(data: data, encoding: .utf8)
        else { return MemoryFailure(code: .invalid, message: "数据编码失败") }
        let raw = userDirectory.path.withCString { dir in json.withCString { qj_memory_write(dir, $0) } }
        return MemoryFailure.decode(take(raw))
    }

    /// 加一个场景或给已有的改名（只写 scenes.json，不校验卡片）；成功返回 nil。
    static func putScene(_ scene: MemoryScene, userDirectory: URL) -> MemoryFailure? {
        let raw = userDirectory.path.withCString { dir in
            scene.id.withCString { id in scene.name.withCString { qj_memory_put_scene(dir, id, $0) } }
        }
        return MemoryFailure.decode(take(raw))
    }

    /// 删一个场景，里面的人挪到默认场景（不校验卡片）；成功返回 nil。
    static func deleteScene(id: String, userDirectory: URL) -> MemoryFailure? {
        let raw = userDirectory.path.withCString { dir in id.withCString { qj_memory_delete_scene(dir, $0) } }
        return MemoryFailure.decode(take(raw))
    }

    /// 取走桥返回的字符串并释放。
    static func take(_ raw: UnsafeMutablePointer<CChar>?) -> String? {
        guard let raw else { return nil }
        defer { qj_string_free(raw) }
        return String(cString: raw)
    }

    static func decode<T: Decodable>(_ json: String?) -> T? {
        guard let data = json?.data(using: .utf8) else { return nil }
        return try? JSONDecoder().decode(T.self, from: data)
    }
}
