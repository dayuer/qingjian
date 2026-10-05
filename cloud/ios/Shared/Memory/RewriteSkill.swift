// 改写用哪个技能的纯值部分（键盘的按钮写什么、技能排哪一项高亮），提出来方便单测。
// 绑定是「全局默认 + 人身上覆盖」：选中的人指定了就用他的，没指定用设置里的默认，两个都认不得用列表第一个。

enum RewriteSkill {
    /// 此刻生效的技能：选中的人身上指定了就用它 → 设置里的默认 → 列表第一个。
    /// 列表为空时 nil：包没打进来到时候，改写整块不出现（见 ScopeDisplay.canRewrite）。
    static func resolve(skills: [Skill], contactSkill: String?, defaultSkill: String) -> Skill? {
        if let contactSkill, let wanted = skills.first(where: { $0.id == contactSkill }) {
            return wanted
        }
        if let fallback = skills.first(where: { $0.id == defaultSkill }) {
            return fallback
        }
        return skills.first
    }

    /// 技能排里的几项：第一项是「用默认」（把人身上的指定清掉），后面是各个技能。
    /// `current` 是选中的人身上指定的那个（nil = 用默认），决定哪一项高亮。
    static func options(skills: [Skill], current: String?) -> [RewriteSkillOption] {
        [RewriteSkillOption(id: nil, title: noOverrideTitle, selected: current == nil)]
            + skills.map {
                RewriteSkillOption(id: $0.id, title: $0.name, selected: $0.id == current)
            }
    }

    static let noOverrideTitle = "用默认"
}
