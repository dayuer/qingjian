// Apple 登录的 nonce：请求里给 Apple 的是 SHA-256 十六进制，原始值交给桥，服务端核对 identity_token 里的 nonce 声明。

import CryptoKit
import Foundation

enum Nonce {
    /// 32 字节随机数的十六进制。SystemRandomNumberGenerator 在 Apple 平台上是密码学安全的。
    static func random() -> String {
        var generator = SystemRandomNumberGenerator()
        return (0..<32)
            .map { _ in String(format: "%02x", UInt8.random(in: .min ... .max, using: &generator)) }
            .joined()
    }

    static func sha256Hex(_ text: String) -> String {
        SHA256.hash(data: Data(text.utf8)).map { String(format: "%02x", $0) }.joined()
    }
}
