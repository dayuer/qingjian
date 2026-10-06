// 候选格宽度的缓存：按「文字 + 是否首选」记，每键只量新出现的词。候选栏与展开面板各持一个。

import UIKit

final class CandidateWidthCache {
    private struct Key: Hashable {
        let text: String

        let highlighted: Bool
    }

    private var widths: [Key: CGFloat] = [:]

    /// 记满就整个清掉重来：候选随拼音换得很快，留着旧的没用。
    private let capacity: Int

    init(capacity: Int = 4096) {
        self.capacity = capacity
    }

    /// `limit`：可用宽度，超长的封顶到它，文字由 UILabel 尾部截断。
    func width(_ text: String, highlighted: Bool, scale: CGFloat, limit: CGFloat) -> CGFloat {
        let key = Key(text: text, highlighted: highlighted)
        let width: CGFloat
        if let cached = widths[key] {
            width = cached
        } else {
            if widths.count >= capacity { widths.removeAll(keepingCapacity: true) }
            width = CandidateWidth.measure(text, highlighted: highlighted, scale: scale)
            widths[key] = width
        }
        return min(width, max(limit, CandidateWidth.minimum))
    }

    var count: Int { widths.count }
}
