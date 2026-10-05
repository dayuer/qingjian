// 全局默认改写技能在壳这边的值：设置里 config.toml 的 [rewrite] skill，经桥的 qj_rewrite_default 取回。
// 读不是用户刚做的动作，NULL 或解析不了时不弹错，一律按缺省（写坏的值由桥在写的时候拦住）。

import Foundation

struct RewriteDefault: Decodable, Equatable {
    /// 技能包 id。
    var skill: String

    /// 缺省技能（与桥的 DEFAULT_SKILL_ID 同一把）。
    static let fallbackSkill = "polish"

    /// 解析桥的 `{"skill":"…"}`；NULL 或解析不了时按缺省。
    static func decode(_ json: String?) -> RewriteDefault {
        guard let data = json?.data(using: .utf8),
              let value = try? JSONDecoder().decode(RewriteDefault.self, from: data)
        else { return RewriteDefault(skill: fallbackSkill) }
        return value
    }
}
