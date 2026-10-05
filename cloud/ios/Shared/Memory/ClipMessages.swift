// 「记一笔」把剪贴板拆成几张卡：微信多选复制是「名字 / 2026年10月05日 09:34 / 正文」一段段排的，去掉名字与时间两行、一条消息一张卡；
// 别的长文按空行分段、拼成每张不超过 200 字的卡。一张卡最多 200 字（MemoryLimits），以前整段截到 200 字，后面的消息就丢了。

import Foundation

enum ClipMessages {
    /// 一次最多记这么多张，免得误复制一大段聊天记录塞满一个人的卡。
    static let maxCards = 10

    static func split(_ text: String) -> [String] {
        let lines = text.replacingOccurrences(of: "\r\n", with: "\n").components(separatedBy: "\n")
        let bodies = wechatBodies(lines) ?? [text]
        return Array(bodies.flatMap(chunks).prefix(maxCards))
    }

    /// 时间行：「2026年10月05日 09:34」或「2026/10/5 09:34」，可带秒。
    static func isTimestamp(_ line: String) -> Bool {
        line.trimmingCharacters(in: .whitespaces)
            .range(of: #"^\d{4}(年\d{1,2}月\d{1,2}日|[/-]\d{1,2}[/-]\d{1,2})\s+\d{1,2}:\d{2}(:\d{2})?$"#, options: .regularExpression) != nil
    }

    /// 第二行起是「非空的名字行 + 时间行」才当微信多选复制，返回去掉头两行的各条正文；不是这个格式返回 nil。
    private static func wechatBodies(_ lines: [String]) -> [String]? {
        let headers = lines.indices.filter { index in
            index > 0 && isTimestamp(lines[index])
                && !lines[index - 1].trimmingCharacters(in: .whitespaces).isEmpty
        }
        guard let first = headers.first, first == firstContentIndex(lines) + 1 else { return nil }
        return headers.enumerated().compactMap { order, header in
            let end = order + 1 < headers.count ? headers[order + 1] - 1 : lines.count
            let body = lines[(header + 1)..<max(header + 1, end)].joined(separator: "\n")
                .trimmingCharacters(in: .whitespacesAndNewlines)
            return body.isEmpty ? nil : body
        }
    }

    private static func firstContentIndex(_ lines: [String]) -> Int {
        lines.firstIndex { !$0.trimmingCharacters(in: .whitespaces).isEmpty } ?? 0
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
