// 设置页的数据：与桥的 `Settings`（JSON）一一对应，字段名经 snake_case 转换。

import Foundation

struct KeyboardSettings: Codable, Equatable {
    var scheme: String

    /// 可选的拼音方案（只读）。
    var schemes: [SchemeChoice]

    var fuzzy: FuzzyOptions

    var traditional: Bool

    var fullWidthPunctuation: Bool

    var learning: Bool

    /// 云联想（经青简 Cloud 补候选），与 Mac 同一个开关。
    var cloudPrediction: Bool

    var domains: [DomainOption]

    var phrases: [Phrase]
}

struct SchemeChoice: Codable, Equatable, Hashable {
    var key: String

    var label: String
}

struct DomainOption: Codable, Equatable, Identifiable {
    var id: String

    var label: String

    var enabled: Bool
}

/// 自定义短语：敲 `code` 时 `text` 固定出现在第 `position` 个候选。
struct Phrase: Codable, Equatable, Hashable {
    var code: String

    var text: String

    var position: Int

    var enabled: Bool
}
