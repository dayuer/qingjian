// 「键盘记住的事」的数据：经桥整份读写 App Group 里的 memory/，改一处写一次。校验（全局置顶不超过 4 个、日期格式）在桥里。
// 任何读写失败都要变成界面上的一句中文（loadError 常驻首页、详情与设置页顶上，message 弹窗），不静默。
// 读写都交给 MemoryWorker 在后台串行做（桥等锁最多 2 秒，不能卡界面），这里只管界面状态：loading / saving、结果回来后更新与弹提示。
// conflict（键盘这期间「记一笔」改过）时 MemoryWorker 重读、用 MemoryMerge 合并上去再写，最多三轮；App 回到前台时重读（SetupView）。
// 目录设数据保护 completeUntilFirstUserAuthentication，不用 complete：锁屏通知里直接回复时键盘也要读卡片给提示。

import Foundation
import Observation

@MainActor
@Observable
final class MemoryStore {
    /// memory/ 的数据保护级别：开机后第一次解锁前读不了，之后锁屏也能读。桥建的子目录与原子写的临时文件继承所在目录的级别。
    nonisolated static let protection = FileProtectionType.completeUntilFirstUserAuthentication

    private(set) var snapshot = MemorySnapshot()

    /// 首页「+ 记一条」记下、还没补上归人的**无主素材**（不绑对象的那一桶，与按对象的那份同形）。
    /// 单独一个文件，随 `reload()` 一起读；读不出时报空表，不至于让首页跟着空掉。
    private(set) var unassigned: [MemoryMaterial] = []

    /// 读不出来的原因（容器不可用、开机后还没解锁过）；有它时首页常驻显示，并且不让写，免得空数据覆盖文件。
    private(set) var loadError: String?

    /// 已经成功读过一次（之前的空列表是真空，不是没读）。
    private(set) var loaded = false

    /// 正在后台读：首页显示加载中。
    private(set) var loading = false

    /// 正在后台写：编辑页显示「正在保存」、按钮置灰；这期间再提交的保存直接拒掉。
    private(set) var saving = false

    /// 要弹给用户的话（写失败、冲突已合并、文件损坏）。
    var message: String?

    /// 开着几个弹出的编辑页；有时由编辑页自己弹 message，TabView 那层不弹（见 MemoryAlert）。
    var editorsOpen = 0

    @ObservationIgnored private let directory: () -> URL?

    @ObservationIgnored private let worker: MemoryWorker

    /// 每开始一次保存加一；读回来时发现期间有保存开始过，就丢掉这次读的结果（保存会自己重读）。
    @ObservationIgnored private var saveGeneration = 0

    init(directory: @escaping () -> URL? = { SharedStore.directory }, backend: MemoryBackend = .bridge) {
        self.directory = directory
        self.worker = MemoryWorker(backend: backend)
    }

    /// 能不能改：容器在、读成功过、现在没有读失败。
    var canEdit: Bool { loaded && loadError == nil }

    /// 最多置顶几个（与桥的 `MAX_PINNED` 一致）；文案在非隔离的 `Wording` 里也要用，所以 nonisolated。
    nonisolated static let pinLimit = 4

    /// 已经置顶了几个（全局最多 [`Self.pinLimit`] 个）。
    var pinnedCount: Int { snapshot.contacts.filter { $0.pinnedAt != nil }.count }

    /// 名单里的人，按置顶与沟通情况排（补上素材时选人用）。
    var people: [MemoryContact] {
        ContactOrder.ordered(snapshot.contacts, used: snapshot.state.used)
    }

    func contact(_ id: String) -> MemoryContact? { snapshot.contacts.first { $0.id == id } }

    func cards(of id: String) -> [MemoryCard] { snapshot.cards[id] ?? [] }

    func reload() async {
        guard let directory = directory() else {
            loadError = Wording.noAppGroup
            return
        }
        // 启动时 .task 与回到前台会连着各读一次：正在读就不再读
        guard !saving, !loading else { return }
        let generation = saveGeneration
        loading = true
        let next = await worker.read(directory)
        loading = false
        guard generation == saveGeneration else { return }
        guard let next else {
            loadError = Wording.unreadable
            return
        }
        apply(next)
        // 无主素材是另一个文件（不绑对象的那一桶），和快照一起读；它读不出只是首页少几条，不影响上面的成败
        unassigned = await worker.unassigned(directory)
    }

    /// 测试用：不经桥直接换数据。
    func replace(with snapshot: MemorySnapshot) {
        self.snapshot = snapshot
        loaded = true
    }

    /// 改一份交给后台整份写回，成功返回 true。正在保存时直接返回 false（不排队，界面那时按钮是灰的）。
    /// 冲突合并在 MemoryWorker 里做；结果回到主线程再更新界面、弹提示。失败时内存里的不改。
    @discardableResult
    func update(
        contactId: String? = nil, forgetting: String? = nil, _ change: (inout MemorySnapshot) -> Void
    ) async -> Bool {
        guard !saving else { return false }
        guard let directory = directory() else {
            loadError = Wording.noAppGroup
            message = Wording.noAppGroup
            return false
        }
        guard canEdit else {
            message = loadError ?? Wording.unreadable
            return false
        }
        var next = snapshot
        change(&next)
        saving = true
        saveGeneration += 1
        let result = await worker.save(
            base: snapshot, next: next, directory: directory, contactId: contactId, forgetting: forgetting)
        saving = false
        switch result {
        case .saved(let written, let latest, let merged, let brokenOnMerge):
            let names = brokenOnMerge.map { id in contact(id)?.name ?? "有个人" }
            apply(latest ?? written)
            var lines = merged ? [Wording.merged] : []
            if !names.isEmpty { lines.append(Wording.broken(names)) }
            if !lines.isEmpty { message = lines.joined(separator: "\n") }
            return true
        case .forgetPartial(let latest):
            let name = contact(forgetting ?? "")?.name ?? "这个人"
            apply(latest)
            message = Wording.forgetPartial(name)
        case .failed(let failure):
            message = Wording.failed(failure)
        case .conflictUnreadable:
            message = Wording.conflictUnreadable
        case .gaveUp(let latest):
            if let latest { apply(latest) }
            message = Wording.conflictGaveUp
        }
        return false
    }

    private func apply(_ next: MemorySnapshot) {
        snapshot = next
        loaded = true
        loadError = nil
        let broken = next.broken.map { id in contact(id)?.name ?? "有个人" }
        if !broken.isEmpty { message = Wording.broken(broken) }
    }

    @discardableResult
    func addContact(_ contact: MemoryContact, cards: [MemoryCard]) async -> Bool {
        await update(contactId: contact.id) {
            $0.contacts.append(contact)
            $0.cards[contact.id] = cards
        }
    }

    @discardableResult
    func saveContact(_ contact: MemoryContact) async -> Bool {
        await update(contactId: contact.id) { snapshot in
            if let index = snapshot.contacts.firstIndex(where: { $0.id == contact.id }) {
                snapshot.contacts[index] = contact
            }
        }
    }

    /// 忘掉这个人：从名单去掉，桥连目录（卡片与分区学习）一起删。
    @discardableResult
    func forget(_ id: String) async -> Bool {
        await update(contactId: id, forgetting: id) {
            $0.contacts.removeAll { $0.id == id }
            $0.cards[id] = nil
        }
    }

    @discardableResult
    func saveCard(_ card: MemoryCard, for id: String) async -> Bool {
        await update(contactId: id) { snapshot in
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
    func deleteCard(_ cardId: String, for id: String) async -> Bool {
        await update(contactId: id) { $0.cards[id]?.removeAll { $0.id == cardId } }
    }

    /// 快速记一条：**不问是谁**，先落进无主素材桶（首页「+ 记一条」）。整理留到事后用 `assign`。
    /// 走的是键盘「记一笔」那同一套素材（切段、2000 字节、200 条上限都一样），只是不绑对象。
    @discardableResult
    func quickNote(_ text: String) async -> Bool {
        let trimmed = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !trimmed.isEmpty else { return false }
        guard let directory = directory() else {
            loadError = Wording.noAppGroup
            message = Wording.noAppGroup
            return false
        }
        guard canEdit else {
            message = loadError ?? Wording.unreadable
            return false
        }
        if let failure = await worker.addUnassigned(directory, text: trimmed, source: .typed) {
            message = Wording.failed(failure)
            return false
        }
        unassigned = await worker.unassigned(directory)
        return true
    }

    /// 把无主素材里的一条归到某个人名下（首页事件行的「补上」）。
    @discardableResult
    func assign(_ clientId: String, to contactId: String) async -> Bool {
        guard let directory = directory() else {
            loadError = Wording.noAppGroup
            message = Wording.noAppGroup
            return false
        }
        guard canEdit else {
            message = loadError ?? Wording.unreadable
            return false
        }
        if let failure = await worker.assign(directory, clientId: clientId, contactId: contactId) {
            message = Wording.failed(failure)
            return false
        }
        // 素材挪到那个人名下了，两边都要重读：无主桶少一条、那个人的「待整理」多一条
        unassigned = await worker.unassigned(directory)
        await reload()
        return true
    }

    /// 开着日子提醒的人今天到 `within` 天后的日子与约定，近的在前。
    func upcoming(within days: Int, now: Date = Date()) -> [MemoryUpcoming] {
        snapshot.contacts.filter(\.remindOn).flatMap { contact in
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
    nonisolated static func protect(_ directory: URL) {
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
