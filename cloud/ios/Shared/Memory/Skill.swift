// 一个改写技能包（桥的 `Skill` 的显示部分）：名字与一句说明。提示词不下发到壳里，壳只用来显示名字。
// 随包走，App 与键盘各读自己包里的那一份：键盘从会话里读（Engine.rewriteSkills），App 从 App 包的 skills/ 读（SkillFiles）。

import Foundation

struct Skill: Codable, Identifiable, Hashable, Sendable {
    /// 技能包 id（小写字母、数字、`-`、`_`）；人身上按它记，定了就不该改。
    let id: String

    /// 键盘按钮与技能排上显示的名字。
    let name: String

    /// 跟在名字后面的一句说明。
    var summary: String = ""

    enum CodingKeys: String, CodingKey {
        case id, name, summary
    }
}

extension Skill {
    /// 缺 `summary` 时按空（桥那边它也是可选的）。
    init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        self.init(
            id: try container.decode(String.self, forKey: .id),
            name: try container.decode(String.self, forKey: .name),
            summary: try container.decodeIfPresent(String.self, forKey: .summary) ?? "")
    }
}
