// 一格候选的宽：文字宽加左右留白，至少 40pt。候选栏与展开面板都按它给格子定宽，格子不再走 Auto Layout 的自适应测量
// （每键量一遍，2026-10-06 真机候选栏每键约 25ms）。字体与留白只在这里定，CandidateCellView 也从这里取。
// 量字用 NSString.size(withAttributes:)，按像素向上取整，与 UILabel 的 intrinsicContentSize 一致（CandidateWidthTests 对过）。
// 放 Shared 的原因同 CandidateStyle：测试 target 测得到。

import UIKit

enum CandidateWidth {
    static let fontSize: CGFloat = 22

    /// 文字左右各留的白。
    static let padding: CGFloat = 10

    static let minimum: CGFloat = 40

    /// 首选加粗一档（CandidateStyle.weight）；accent 只改字色，不影响宽度。
    static func font(highlighted: Bool) -> UIFont {
        .systemFont(ofSize: fontSize, weight: CandidateStyle.weight(highlighted: highlighted))
    }

    /// `scale`：屏幕像素与 pt 之比，取整到像素与 UILabel 一致。
    static func measure(_ text: String, highlighted: Bool, scale: CGFloat) -> CGFloat {
        let raw = (text as NSString).size(withAttributes: [.font: font(highlighted: highlighted)]).width
        let textWidth = (raw * scale).rounded(.up) / scale
        return max(textWidth + padding * 2, minimum)
    }
}
