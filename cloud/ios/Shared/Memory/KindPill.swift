// 改一条里「是什么」的胶囊（设计稿 .opts）：五种按日子、约定、喜好、近况、其他排，选中的黑底白字，其余线框灰字，全是中性色。

import SwiftUI

struct KindPill: Equatable, Identifiable {
    let kind: MemoryCard.Kind

    let selected: Bool

    var id: MemoryCard.Kind { kind }

    var title: String { kind.title }

    /// 选中时有实底（ink），没选中只有线框。
    var filled: Bool { selected }

    var outlined: Bool { !selected }

    /// 文字的颜色角色；选中时是底色上的反色（paper），不算角色，为 nil。
    var textRole: ColorRole? { selected ? nil : .ink2 }

    var foreground: Color { selected ? Color(UIColor.systemBackground) : Theme.ink2 }

    var background: Color { selected ? Theme.ink : .clear }

    static func pills(selected: MemoryCard.Kind) -> [KindPill] {
        MemoryCard.Kind.allCases.map { KindPill(kind: $0, selected: $0 == selected) }
    }
}
