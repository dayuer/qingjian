// 设置的数据：与桥的 `Settings`（JSON）一一对应，字段名经 snake_case 转换。App 的键盘设置页与键盘共用
// （键盘改全局默认技能时也要整份读出来再写回，缺字段会让桥拒掉）。

import Foundation

struct KeyboardSettings: Codable, Equatable {
    var scheme: String

    /// 可选的拼音方案（只读）。
    var schemes: [SchemeChoice]

    var fuzzy: FuzzyOptions

    var traditional: Bool

    var fullWidthPunctuation: Bool

    var learning: Bool

    /// 云联想（经素笺云 补候选），与 Mac 同一个开关。
    var cloudPrediction: Bool

    var domains: [DomainOption]

    var phrases: [Phrase]

    /// 改写用的默认技能（技能包 id）；某个人身上指定了就用他的。设置页里没有这一项，改由键盘上的技能排写。
    var rewriteSkill: String
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
