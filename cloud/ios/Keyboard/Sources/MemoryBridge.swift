// 调桥的本地记忆接口（qj_scope_* / qj_memory_*）：Engine 的扩展，用同一个会话指针，只在主线程上用。

import Foundation
import QingjianBridge

extension Engine {
    /// 当前对象与各人上次被选中的时间；会话没有记忆目录时为 nil。
    var scope: MemoryScope? { MemoryFiles.decode(take(qj_scope_get(session))) }

    /// 切当前对象：`.keep` 保持现在选的人不变（幂等），`.nobody` 明确不指定。
    func setScope(pick: ScopePick) {
        Self.withOptionalCString(pick.argument) { qj_scope_set(session, $0) }
    }

    /// 提示行要显示的；私密输入或没选对象时为 nil。
    var memoryHint: MemoryHint? { MemoryFiles.decode(take(qj_memory_hint(session))) }

    /// 「知道了」：today 为真当天不再出，为假 10 分钟内不再出。
    func dismissHint(_ cardId: String, today: Bool) {
        cardId.withCString { qj_memory_dismiss(session, $0, today) }
    }

    /// 对象卡面板：今日相关最多 3 张。
    func memoryCards(_ contactId: String) -> [MemoryCard] {
        let raw = contactId.withCString { qj_memory_cards(session, $0) }
        return MemoryFiles.decode(take(raw)) ?? []
    }

    /// 宿主换了输入框：桥清掉最近上屏的字，免得 A 聊天里打的字在 B 里触发提示（键盘收起时 flush 也会清）。
    func resetContext() { qj_reset_context(session) }

    /// 「记一笔」：桥把原话存成待整理素材（超过 2000 字节的切成几条）。`source` 是 clipboard（剪贴板确认条）或 typed（手写）。
    /// 桥返回 NULL 表示成功（这里得到 nil），失败才返回 `{"code","message"}`：invalid 是没有这个人或没有文字，
    /// material_limit 是切出的条数加上没整理的超过 200 条（带 remaining / needed），io 是素材读不了（开机后还没解锁过），此时桥什么都没写。
    func memoryNote(_ contactId: String, text: String, source: String) -> MemoryFailure? {
        let raw = contactId.withCString { c in
            text.withCString { t in source.withCString { qj_memory_note(session, c, t, $0) } }
        }
        return MemoryFailure.decode(take(raw))
    }

    /// 拿不到锁排队的记一笔，补写时被拒绝的条数（按原因）；取一次桥就清零，没有时为 nil。
    func memoryDropped() -> DroppedNotes? { MemoryFiles.decode(take(qj_memory_dropped(session))) }

    /// 键盘里新建一个对象，称呼先按 TA（App 里能改）；建好返回 id，名字为空时失败。
    func addContact(name: String) -> Result<String, MemoryFailure> {
        let raw = name.withCString { qj_memory_add_contact(session, $0, nil) }
        return ContactAdd.parse(take(raw))
    }

    /// 可用的改写技能（随包的技能包，会话打开时读一次）；包没打进来时为空。
    var rewriteSkills: [Skill] { MemoryFiles.decode(take(qj_rewrite_skills(session))) ?? [] }

    /// 给这个人指定 / 清掉改写技能（nil = 回到设置里的默认）；成功给 nil。
    func setContactSkill(_ contactId: String, skillId: String?) -> MemoryFailure? {
        guard let userDirectory else {
            return MemoryFailure(code: .invalid, message: "记忆目录不可用")
        }
        let raw = userDirectory.path.withCString { dir in
            contactId.withCString { id in
                Self.withOptionalCString(skillId) { qj_memory_contact_skill_set(dir, id, $0) }
            }
        }
        return MemoryFailure.decode(take(raw))
    }
}
