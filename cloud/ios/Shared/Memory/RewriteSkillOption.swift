// 技能排里的一项。`id` 为 nil 表示「用默认」：清掉人身上的指定，回到设置里的默认。

struct RewriteSkillOption: Equatable, Identifiable {
    let id: String?

    let title: String

    let selected: Bool
}
