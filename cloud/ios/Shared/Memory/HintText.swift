// 提示行的文字：命中的词加粗（桥的 Hint 不带命中词，由 Swift 侧按卡片关键词找），以及提示来源的小标签（手动卡显示「你写的」，灰字不可点）。

import Foundation

enum HintText {
    /// 在 `text` 里把 `emphasis` 中第一个出现的词加粗；一个都没出现、词为空时原样返回。
    static func attributed(text: String, emphasis: [String]) -> AttributedString {
        var result = AttributedString(text)
        for word in emphasis where !word.isEmpty {
            guard let found = text.range(of: word),
                  let range = Range(found, in: result)
            else { continue }
            result[range].inlinePresentationIntent = .stronglyEmphasized
            break
        }
        return result
    }

    /// 要加粗的词：日子 / 约定提醒取卡片文字（「明天是她的**生日**」），匹配提示取卡片关键词。
    static func emphasis(for hint: MemoryHint, card: MemoryCard?) -> [String] {
        guard let card else { return [] }
        return hint.reason == .today ? [card.text] : card.keywords
    }

    /// 提示右边的来源小字：手动卡是「你写的」（2A 的卡都是手动卡），云端卡不写。
    static func sourceLabel(for source: String?) -> String? {
        (source ?? "manual") == "manual" ? "你写的" : nil
    }
}
