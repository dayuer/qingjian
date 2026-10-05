// 键盘选择面板与工具栏快速切人列取哪些人、按什么顺序：置顶的先，其余按「沟通情况」——
// 键盘里上次选中这个人的时间（`state.used`），没选过的按认识时间。App 与键盘共用这一个纯值。

import Foundation

enum ContactOrder {
    /// 选择面板最多摆几个人：键区里 10 格三行，减掉「不指定」与「新对象」两格。
    static let panelCount = 8

    /// 点牌子右半、工具栏里横着列几个（放不下就裁掉）。
    static let quickPickCount = 6

    /// 沟通情况：键盘里上次选中这个人的时间；没选过的用认识时间，等于排在选过的人后面。
    static func score(_ contact: MemoryContact, used: [String: Int64]) -> Int64 {
        used[contact.id] ?? contact.createdAt
    }

    /// 排好并取前 `limit` 个（`limit` 为 nil 就是全部）：置顶的先（早置顶的在前），
    /// 其余按沟通情况降序、认识时间降序、名字。
    static func ordered(
        _ people: [MemoryContact], used: [String: Int64], limit: Int? = nil
    ) -> [MemoryContact] {
        let sorted = people.sorted { a, b in
            switch (a.pinnedAt, b.pinnedAt) {
            case let (pa?, pb?) where pa != pb: return pa < pb
            case (_?, nil): return true
            case (nil, _?): return false
            default: break
            }
            let left = score(a, used: used)
            let right = score(b, used: used)
            if left != right { return left > right }
            return a.name.localizedStandardCompare(b.name) == .orderedAscending
        }
        guard let limit else { return sorted }
        return Array(sorted.prefix(limit))
    }
}
