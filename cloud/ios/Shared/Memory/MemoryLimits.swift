// 手写卡的上限，与桥（proto 的 MAX_CARD_TEXT_CHARS / MAX_CARD_KEYWORDS）一致：文字最多 200 字、关键词最多 8 个、每个 2–8 字。
// 「字」按 Unicode 标量数，与 Rust 的 chars().count() 一样（中文、单个 emoji 各算一个）；桥写入前还会再校验一遍。

enum MemoryLimits {
    static let maxTextChars = 200

    static let maxKeywords = 8

    static let keywordChars = 2...8

    /// 文字的字数（Unicode 标量数）。
    static func count(_ text: String) -> Int { text.unicodeScalars.count }

    /// 超过 200 字就截掉后面的。
    static func clampText(_ text: String) -> String {
        guard count(text) > maxTextChars else { return text }
        return String(String.UnicodeScalarView(text.unicodeScalars.prefix(maxTextChars)))
    }

    /// 编辑器里的计数，如「137 / 200」。
    static func counter(_ text: String) -> String { "\(count(text)) / \(maxTextChars)" }

    /// 这个关键词能不能加：去掉首尾空白后 2–8 字、还没满 8 个、没重复。
    static func canAdd(_ keyword: String, to keywords: [String]) -> Bool {
        let trimmed = keyword.trimmingCharacters(in: .whitespacesAndNewlines)
        return keywords.count < maxKeywords && keywordChars.contains(count(trimmed)) && !keywords.contains(trimmed)
    }
}
