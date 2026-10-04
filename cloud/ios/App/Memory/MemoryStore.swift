// 「键盘记住的事」的数据：经桥整份读写 App Group 里的 memory/，改一处写一次。校验（恋爱场景最多 8 个人、日期格式）在桥里。
// 任何读写失败都要变成界面上的一句中文（loadError 常驻首页，message 弹窗），不静默：用户真机上找不到页面、加不了人就是因为没反馈。
// conflict（键盘这期间「记一笔」改过）时重读、用 MemoryMerge 合并上去再写，最多三轮；App 回到前台时重读（SetupView）。
// 目录设数据保护 completeUntilFirstUserAuthentication，不用 complete：锁屏通知里直接回复时键盘也要读卡片给提示。

import Foundation
import Observation

@MainActor
@Observable
final class MemoryStore {
    static let contactLimit = ScopeDisplay.maxContacts

    /// memory/ 的数据保护级别：开机后第一次解锁前读不了，之后锁屏也能读。桥建的子目录与原子写的临时文件继承所在目录的级别。
    static let protection = FileProtectionType.completeUntilFirstUserAuthentication

    private(set) var snapshot = MemorySnapshot()

    /// 读不出来的原因（容器不可用、锁屏）；有它时首页常驻显示，并且不让写，免得空数据覆盖文件。
    private(set) var loadError: String?

    /// 已经成功读过一次（之前的空列表是真空，不是没读）。
    private(set) var loaded = false

    /// 要弹给用户的话（写失败、冲突已合并、文件损坏）。
    var message: String?

    /// 开着几个弹出的编辑页；有时由编辑页自己弹 message，TabView 那层不弹（见 MemoryAlert）。
    var editorsOpen = 0

    @ObservationIgnored private let directory: () -> URL?

    @ObservationIgnored private let backend: MemoryBackend

    init(directory: @escaping () -> URL? = { SharedStore.directory }, backend: MemoryBackend = .bridge) {
        self.directory = directory
        self.backend = backend
    }

    /// 能不能改：容器在、读成功过、现在没有读失败。
    var canEdit: Bool { loaded && loadError == nil }

    /// 恋爱场景的人。
    var people: [MemoryContact] { snapshot.contacts.filter { $0.scene == MemoryScope.dating } }

    func contact(_ id: String) -> MemoryContact? { snapshot.contacts.first { $0.id == id } }

    func cards(of id: String) -> [MemoryCard] { snapshot.cards[id] ?? [] }

    func reload() {
        guard let directory = directory() else {
            loadError = Text.noAppGroup
            return
        }
        Self.protect(directory.appendingPathComponent("memory", isDirectory: true))
        guard let next = backend.read(directory) else {
            loadError = Text.unreadable
            return
        }
        snapshot = next
        loaded = true
        loadError = nil
        let broken = next.broken.map { id in contact(id)?.name ?? "有个人" }
        if !broken.isEmpty { message = Text.broken(broken) }
    }

    /// 测试用：不经桥直接换数据。
    func replace(with snapshot: MemorySnapshot) {
        self.snapshot = snapshot
        loaded = true
    }

    /// 改一份再整份写回，成功返回 true。冲突时重读、三方合并后再写（最多三轮），合并成功也告诉用户一声；
    /// 写成功后重读一遍拿新的修订号。别的失败不改内存里的，弹原因。
    @discardableResult
    func update(_ change: (inout MemorySnapshot) -> Void) -> Bool {
        guard let directory = directory() else {
            loadError = Text.noAppGroup
            message = Text.noAppGroup
            return false
        }
        guard canEdit else {
            message = loadError ?? Text.unreadable
            return false
        }
        var base = snapshot
        var next = snapshot
        change(&next)
        var merged = false
        for _ in 0..<3 {
            next.broken = []
            guard let failure = backend.write(next, directory) else {
                snapshot = next
                reload()
                if merged { message = Text.merged }
                return true
            }
            guard failure.code == .conflict else {
                message = Text.failed(failure)
                return false
            }
            guard let remote = backend.read(directory) else {
                message = Text.conflictUnreadable
                return false
            }
            next = MemoryMerge.merge(base: base, local: next, remote: remote)
            base = remote
            merged = true
        }
        reload()
        message = Text.conflictGaveUp
        return false
    }

    @discardableResult
    func addContact(_ contact: MemoryContact, cards: [MemoryCard]) -> Bool {
        update {
            $0.contacts.append(contact)
            $0.cards[contact.id] = cards
        }
    }

    @discardableResult
    func saveContact(_ contact: MemoryContact) -> Bool {
        update { snapshot in
            if let index = snapshot.contacts.firstIndex(where: { $0.id == contact.id }) {
                snapshot.contacts[index] = contact
            }
        }
    }

    /// 忘掉这个人：从名单去掉，桥连目录（卡片与分区学习）一起删。
    @discardableResult
    func forget(_ id: String) -> Bool {
        update {
            $0.contacts.removeAll { $0.id == id }
            $0.cards[id] = nil
        }
    }

    @discardableResult
    func saveCard(_ card: MemoryCard, for id: String) -> Bool {
        update { snapshot in
            var list = snapshot.cards[id] ?? []
            if let index = list.firstIndex(where: { $0.id == card.id }) {
                list[index] = card
            } else {
                list.append(card)
            }
            snapshot.cards[id] = list
        }
    }

    @discardableResult
    func deleteCard(_ cardId: String, for id: String) -> Bool {
        update { $0.cards[id]?.removeAll { $0.id == cardId } }
    }

    /// 开着日子提醒的人今天到 `within` 天后的日子与约定，近的在前。
    func upcoming(within days: Int, now: Date = Date()) -> [MemoryUpcoming] {
        people.filter(\.remindOn).flatMap { contact in
            cards(of: contact.id).compactMap { card -> MemoryUpcoming? in
                guard let away = card.daysAway(now: now), (0...days).contains(away) else { return nil }
                return MemoryUpcoming(contact: contact, card: card, days: away)
            }
        }
        .sorted { ($0.days, $0.card.text) < ($1.days, $1.card.text) }
    }

    /// 导出为文本：名字、认识几天，按类分组的卡片。
    func exportText(_ id: String, now: Date = Date()) -> String {
        guard let contact = contact(id) else { return "" }
        var lines = ["\(contact.name)（认识 \(contact.knownDays(now: now)) 天）"]
        for kind in MemoryCard.Kind.allCases {
            let list = cards(of: id).filter { $0.kind == kind }
            guard !list.isEmpty else { continue }
            lines.append("")
            lines.append(kind.title)
            for card in list {
                lines.append("· \(card.text)" + (card.when.map { " \($0)" } ?? ""))
            }
        }
        return lines.joined(separator: "\n")
    }

    /// 建好 memory/ 并设 `protection`：第一次（目录还不是这一档时）把整个目录树递归设一遍，之后新建的文件继承目录的属性。
    /// 键盘先于 App 建目录时用的是容器缺省级别，也就是这一档。
    static func protect(_ directory: URL) {
        let manager = FileManager.default
        try? manager.createDirectory(at: directory, withIntermediateDirectories: true)
        let current = (try? manager.attributesOfItem(atPath: directory.path))?[.protectionKey] as? FileProtectionType
        guard current != protection else { return }
        let items = manager.enumerator(at: directory, includingPropertiesForKeys: nil)?
            .compactMap { $0 as? URL } ?? []
        for url in [directory] + items {
            try? manager.setAttributes([.protectionKey: protection], ofItemAtPath: url.path)
        }
    }
}
