// App 写回遇到 conflict（键盘这期间「记一笔」改过）时的三方合并：base 是 App 改之前读的快照，local 是 App 改完的，
// remote 是刚重读的。对象与卡片都以 id 为键：local 新加的加上、local 删掉的删掉、local 改过的用 local 的，其余照 remote
// （键盘新记的卡就这样留下来）；同一张卡两边都改了，touchedAt 新的赢（一样新用 local）；对象两边都改了用 local。修订号用 remote 的。

enum MemoryMerge {
    static func merge(base: MemorySnapshot, local: MemorySnapshot, remote: MemorySnapshot) -> MemorySnapshot {
        var result = remote
        result.broken = []
        result.contacts = mergeList(base: base.contacts, local: local.contacts, remote: remote.contacts) { mine, _ in mine }
        result.cards = [:]
        for contact in result.contacts {
            let id = contact.id
            result.cards[id] = mergeList(
                base: base.cards[id] ?? [], local: local.cards[id] ?? [], remote: remote.cards[id] ?? []
            ) { mine, theirs in mine.touchedAt >= theirs.touchedAt ? mine : theirs }
        }
        return result
    }

    /// 以 id 为键的三方合并，顺序按 remote，local 新加的接在后面。
    private static func mergeList<T: Identifiable & Equatable>(
        base: [T], local: [T], remote: [T], bothChanged: (T, T) -> T
    ) -> [T] where T.ID == String {
        let original = Dictionary(base.map { ($0.id, $0) }, uniquingKeysWith: { first, _ in first })
        let mine = Dictionary(local.map { ($0.id, $0) }, uniquingKeysWith: { first, _ in first })
        var result: [T] = []
        for theirs in remote {
            switch (original[theirs.id], mine[theirs.id]) {
            case (.some, nil):
                continue
            case (.some(let before), .some(let edited)) where edited != before:
                result.append(theirs == before ? edited : bothChanged(edited, theirs))
            case (nil, .some(let edited)):
                result.append(theirs == edited ? edited : bothChanged(edited, theirs))
            default:
                result.append(theirs)
            }
        }
        let remoteIDs = Set(remote.map(\.id))
        result += local.filter { original[$0.id] == nil && !remoteIDs.contains($0.id) }
        return result
    }
}
