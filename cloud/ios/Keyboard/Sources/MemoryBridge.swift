// 调桥的本地记忆接口（qj_scope_* / qj_memory_*）：Engine 的扩展，用同一个会话指针，只在主线程上用。

import Foundation
import QingjianBridge

extension Engine {
    /// 当前场景与对象；会话没有记忆目录时为 nil。
    var scope: MemoryScope? { MemoryFiles.decode(take(qj_scope_get(session))) }

    /// 切场景与对象：`.last` 回到这个场景上次选的人（桥记在 state.json 的 last 里），`.nobody` 明确不指定。
    func setScope(scene: String, pick: ScopePick) {
        scene.withCString { s in
            Self.withOptionalCString(pick.argument) { qj_scope_set(session, s, $0) }
        }
    }

    /// 提示行要显示的；私密输入、工作场景、没选对象时为 nil。
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

    /// 键盘里在 `scene` 新建一个对象，称呼先按 TA（App 里能改）；建好返回 id，这个场景满 8 个时失败（文案带场景名）。
    func addContact(name: String, scene: String) -> Result<String, MemoryFailure> {
        let raw = name.withCString { n in scene.withCString { qj_memory_add_contact(session, n, nil, $0) } }
        return ContactAdd.parse(take(raw))
    }
}
