// 「记一笔」把剪贴板拆成几条素材：原话原样保留（微信多选复制的名字、时间都留着，交给素笺云的大模型整理时要用），
// 每条不超过 2000 字节（UTF-8，与桥的 MAX_MEMORY_TEXT_BYTES 一致），所以一次复制通常就是一条。
// 超过的按空行分段、相邻的段用空行拼回去装满一条；单段还超的按字节硬切，切点落在字符（字形簇）边界上，一个字不丢。
// 桥存素材时还会按同样的上限再切一遍，这里先切好是为了确认条上显示「· n 条」。

import Foundation

enum ClipMessages {
    /// 一条素材最多多少字节（UTF-8）。
    static let maxBytes = 2000

    /// 一次最多记这么多条，免得误复制一大段聊天记录塞满一个人的待整理（每人最多 200 条）。
    static let maxPieces = 10

    static func split(_ text: String) -> [String] {
        Array(pieces(text.replacingOccurrences(of: "\r\n", with: "\n")).prefix(maxPieces))
    }

    private static func pieces(_ body: String) -> [String] {
        let text = body.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty else { return [] }
        guard text.utf8.count > maxBytes else { return [text] }
        var pieces: [String] = []
        var current = ""
        for paragraph in text.components(separatedBy: "\n\n").map({ $0.trimmingCharacters(in: .whitespacesAndNewlines) })
            where !paragraph.isEmpty {
            for piece in hardCut(paragraph) {
                let joinedBytes = current.isEmpty ? piece.utf8.count : current.utf8.count + 2 + piece.utf8.count
                if joinedBytes <= maxBytes {
                    current = current.isEmpty ? piece : current + "\n\n" + piece
                } else {
                    pieces.append(current)
                    current = piece
                }
            }
        }
        if !current.isEmpty { pieces.append(current) }
        return pieces
    }

    /// 按字节切成每块不超过 maxBytes 的几块；按字形簇累加，emoji 组合不会被切开。
    private static func hardCut(_ text: String) -> [String] {
        var pieces: [String] = []
        var current = ""
        var bytes = 0
        for character in text {
            let size = character.utf8.count
            if bytes + size > maxBytes, !current.isEmpty {
                pieces.append(current)
                current = ""
                bytes = 0
            }
            current.append(character)
            bytes += size
        }
        if !current.isEmpty { pieces.append(current) }
        return pieces
    }
}
