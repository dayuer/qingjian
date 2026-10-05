// 「记一笔」把剪贴板拆成几张卡：原话原样保留（微信多选复制的名字、时间都留着，交给素笺云的大模型整理时要用），
// 按空行分段、拼成每张不超过 200 字的卡。一张卡最多 200 字（MemoryLimits），以前整段截到 200 字，后面的就丢了。
// 过渡做法：以后原话存成「待整理」素材、由云端每日整理成卡（见 2B / 2C 计划），这里的切分随之改掉。

import Foundation

enum ClipMessages {
    /// 一次最多记这么多张，免得误复制一大段聊天记录塞满一个人的卡。
    static let maxCards = 10

    static func split(_ text: String) -> [String] {
        // 原话原样保留（名字、时间都留着），以后交给素笺云的大模型整理成卡；这里只为 200 字的卡上限按段切开
        Array(chunks(text.replacingOccurrences(of: "\r\n", with: "\n")).prefix(maxCards))
    }

    /// 超过 200 字的按空行分段拼成几张，单段还超的按 200 字硬切。
    private static func chunks(_ body: String) -> [String] {
        let text = body.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !text.isEmpty else { return [] }
        guard MemoryLimits.count(text) > MemoryLimits.maxTextChars else { return [text] }
        var cards: [String] = []
        var current = ""
        for paragraph in text.components(separatedBy: "\n\n").map({ $0.trimmingCharacters(in: .whitespacesAndNewlines) })
            where !paragraph.isEmpty {
            for piece in hardCut(paragraph) {
                let joined = current.isEmpty ? piece : current + "\n" + piece
                if MemoryLimits.count(joined) <= MemoryLimits.maxTextChars {
                    current = joined
                } else {
                    cards.append(current)
                    current = piece
                }
            }
        }
        if !current.isEmpty { cards.append(current) }
        return cards
    }

    private static func hardCut(_ text: String) -> [String] {
        var scalars = Substring.UnicodeScalarView(text.unicodeScalars)
        var pieces: [String] = []
        while !scalars.isEmpty {
            pieces.append(String(String.UnicodeScalarView(scalars.prefix(MemoryLimits.maxTextChars))))
            scalars = scalars.dropFirst(MemoryLimits.maxTextChars)
        }
        return pieces
    }
}
