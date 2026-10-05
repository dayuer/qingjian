// 点「记一笔」后进哪条路：剪贴板有新文字就出「记到 X」确认条，剪贴板空或和上次处理过的一样就进手写。
// 「上次处理过的」存摘要不存原文：扩展的 UserDefaults 不加密，卡片文字不该落在那里。

import CryptoKit
import Foundation

enum NoteEntry: Equatable {
    /// 出确认条，带拆好的几条素材（ClipMessages：每条不超过 2000 字节，一次复制通常就是一条）。
    case clipboard([String])

    case compose

    static func decide(clipboard: String?, lastHandledDigest: String?) -> NoteEntry {
        guard let text = clipboard.map(trimmed), !text.isEmpty, digest(text) != lastHandledDigest else {
            return .compose
        }
        let pieces = ClipMessages.split(text)
        return pieces.isEmpty ? .compose : .clipboard(pieces)
    }

    /// 剪贴板文字（去掉首尾空白后）的 SHA-256，十六进制。
    static func digest(_ text: String) -> String {
        SHA256.hash(data: Data(trimmed(text).utf8)).map { String(format: "%02x", $0) }.joined()
    }

    private static func trimmed(_ text: String) -> String {
        text.trimmingCharacters(in: .whitespacesAndNewlines)
    }
}
