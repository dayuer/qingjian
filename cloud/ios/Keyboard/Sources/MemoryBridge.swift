// 调桥的本地记忆接口（qj_scope_* / qj_memory_*）：Engine 的扩展，用同一个会话指针，只在主线程上用。

import Foundation
import QingjianBridge

extension Engine {
    /// 当前场景与对象；会话没有记忆目录时为 nil。
    var scope: MemoryScope? { MemoryFiles.decode(take(qj_scope_get(session))) }

    func setScope(scene: String, contactId: String?) {
        scene.withCString { s in
            Self.withOptionalCString(contactId) { qj_scope_set(session, s, $0) }
        }
    }

    /// 提示行要显示的；私密输入、非恋爱场景、没选对象时为 nil。
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

    /// 「记一笔」。桥返回 NULL 表示成功（这里得到 nil），失败才返回 `{"code","message"}`：
    /// invalid 是没有这个人或没有文字，io 是卡片读不了（开机后还没解锁过），此时桥什么都没写。
    func memoryNote(_ contactId: String, text: String) -> MemoryFailure? {
        let raw = contactId.withCString { c in text.withCString { qj_memory_note(session, c, $0) } }
        return MemoryFailure.decode(take(raw))
    }
}
