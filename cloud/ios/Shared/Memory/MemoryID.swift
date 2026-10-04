// 新对象、新卡片的 id：16 字节随机数的小写十六进制，与桥的 new_id 同格式（桥写入时会校验）。

import Foundation
import Security

enum MemoryID {
    static func make() -> String {
        var bytes = [UInt8](repeating: 0, count: 16)
        if SecRandomCopyBytes(kSecRandomDefault, bytes.count, &bytes) != errSecSuccess {
            bytes = withUnsafeBytes(of: UUID().uuid) { Array($0) }
        }
        return bytes.map { String(format: "%02x", $0) }.joined()
    }
}
