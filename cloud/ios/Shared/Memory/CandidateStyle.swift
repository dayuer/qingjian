// 候选的一格长什么样：底色、字色、字重。候选栏（UIKit 的 CandidateBarView）与展开面板
// （CandidatePanelView）共用这一套，两边不能各写各的。
//
// 放 Shared 是为了能被测试 target 测到：Keyboard/Sources 不进测试 target（只有 EngineDisplay 与
// CandidateItem 两个例外），而 Shared 在 App target 的 sources 里。
// 只回 Color / UIFont.Weight，由键盘侧转成 UIColor 用；键盘一律系统字体，不碰 AppFont。

import SwiftUI
import UIKit

enum CandidateStyle {
    /// 首选那格的灰底（照 iOS 26 自带键盘实测：浅色淡黑、深色淡白）。
    static let highlightFill = Color(UIColor { traits in
        traits.userInterfaceStyle == .dark
            ? UIColor(white: 1, alpha: 0.18) : UIColor(white: 0, alpha: 0.07)
    })

    /// 底色：只有首选带灰底。
    static func background(highlighted: Bool) -> Color? {
        highlighted ? highlightFill : nil
    }

    /// 字色：首选且 accent（恋爱、日常选了人）用强调色；大模型给的次一级；其余正文色。
    /// accent 只在首选时才由调用方传 true（`KeyboardModel.accentFirstCandidate`），这里再挡一道。
    static func role(highlighted: Bool, accent: Bool, cloud: Bool) -> ColorRole {
        if highlighted && accent { return ColorUsage.firstCandidate.role }
        return cloud ? .ink2 : .ink
    }

    /// 字重：首选加粗一档，其余常规。回 UIKit 的字重——只有键盘用它（App 侧的字体走 AppFont）。
    static func weight(highlighted: Bool) -> UIFont.Weight {
        highlighted ? .medium : .regular
    }
}
