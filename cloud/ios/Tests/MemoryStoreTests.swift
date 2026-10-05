// App 侧「键盘记住的事」的数据层：本周挑卡、写回冲突的三方合并、导出文本，每条写入失败路径（容器拿不到、
// 读不出、锁超时、冲突、桥报错、忘掉只做了一半）都落成界面上能显示的中文，以及后台写时的「保存中」与重复提交。桥用假的 MemoryBackend 代替。

import Foundation
import XCTest
@testable import QingjianCloud

@MainActor
final class MemoryStoreTests: XCTestCase {
    private let contactId = "0123456789abcdef0123456789abcdef"

    private let directory = URL(fileURLWithPath: NSTemporaryDirectory(), isDirectory: true)
        .appendingPathComponent("memory-store-tests", isDirectory: true)

    private func card(_ id: String, _ text: String, touched: Int64) -> MemoryCard {
        MemoryCard(
            id: id, kind: .other, text: text, keywords: [], when: nil, source: "manual", confirmed: true,
            createdAt: 0, touchedAt: touched)
    }

    private func person(_ name: String = "小美") -> MemoryContact {
        MemoryContact(id: contactId, name: name, pronoun: .ta, createdAt: 0)
    }

    private func sampleSnapshot() -> MemorySnapshot {
        var snapshot = MemorySnapshot()
        snapshot.contacts = [person()]
        snapshot.cards[contactId] = [card("a", "原来的", touched: 1)]
        snapshot.revs[contactId] = 1
        return snapshot
    }

    /// 假桥：读返回 `disk`，写按 `writeResults` 依次给结果（用完后都算成功），成功时把写的内容落到 `disk`。
    /// 读写在 MemoryWorker 的后台队列上调用，状态用锁保护；`gate` 设了时每次写先等它放行，用来测「保存中」。
    private final class FakeBridge: @unchecked Sendable {
        private let lock = NSLock()

        private var _disk: MemorySnapshot?

        private var _writeResults: [MemoryFailure?] = []

        private var _writes: [MemorySnapshot] = []

        private var _reads = 0

        private var _wroteOnMain = false

        var gate: DispatchSemaphore?

        init(disk: MemorySnapshot?) { _disk = disk }

        private func locked<T>(_ body: () -> T) -> T {
            lock.lock()
            defer { lock.unlock() }
            return body()
        }

        var disk: MemorySnapshot? {
            get { locked { _disk } }
            set { locked { _disk = newValue } }
        }

        var writeResults: [MemoryFailure?] {
            get { locked { _writeResults } }
            set { locked { _writeResults = newValue } }
        }

        var writes: [MemorySnapshot] { locked { _writes } }

        var reads: Int { locked { _reads } }

        var wroteOnMain: Bool { locked { _wroteOnMain } }

        var backend: MemoryBackend {
            MemoryBackend(
                read: { _ in
                    self.locked {
                        self._reads += 1
                        return self._disk
                    }
                },
                write: { snapshot, _ in
                    self.gate?.wait()
                    return self.locked {
                        self._wroteOnMain = self._wroteOnMain || Thread.isMainThread
                        self._writes.append(snapshot)
                        let result = self._writeResults.isEmpty ? nil : self._writeResults.removeFirst()
                        if result == nil { self._disk = snapshot }
                        return result
                    }
                })
        }
    }

    private func store(_ bridge: FakeBridge, directory: URL? = nil) async -> MemoryStore {
        let dir = directory ?? self.directory
        let store = MemoryStore(directory: { dir }, backend: bridge.backend)
        await store.reload()
        return store
    }

    // 本周与今天

    func testUpcomingPicksDatesWithinRange() async {
        let store = MemoryStore(directory: { nil }, backend: FakeBridge(disk: nil).backend)
        let now = MemoryDate.parse("2026-10-04")!
        var snapshot = MemorySnapshot()
        snapshot.contacts = [person()]
        snapshot.cards[contactId] = [
            MemoryCard.new(kind: .promise, text: "看电影", when: "2026-10-09", keywords: []),
            MemoryCard.new(kind: .date, text: "生日", when: "1998-10-05", keywords: []),
            MemoryCard.new(kind: .date, text: "纪念日", when: "2026-10-12", keywords: []),
            MemoryCard.new(kind: .preference, text: "草莓", when: "2026-10-04", keywords: []),
        ]
        store.replace(with: snapshot)
        XCTAssertEqual(store.upcoming(within: 3, now: now).map(\.card.text), ["生日"])
        XCTAssertEqual(store.upcoming(within: 6, now: now).map(\.card.text), ["生日", "看电影"])
        XCTAssertEqual(store.upcoming(within: 6, now: now).first?.text, "明天是TA的生日")
        XCTAssertEqual(store.upcoming(within: 6, now: now).last?.dayLabel, "5 天后 · 10-09")
    }

    func testUpcomingCardTitles() async {
        let now = MemoryDate.parse("2026-10-04")!
        let birthday = MemoryUpcoming(
            contact: person(), card: MemoryCard.new(kind: .date, text: "生日", when: "1998-10-05", keywords: []), days: 1)
        XCTAssertEqual(birthday.title, "小美生日")
        XCTAssertEqual(birthday.shortDay, "明天")
        XCTAssertEqual(birthday.card.monthDay(now: now), "10.05")
        let movie = MemoryUpcoming(
            contact: person(), card: MemoryCard.new(kind: .promise, text: "看电影", when: "2026-10-07", keywords: []), days: 3)
        XCTAssertEqual(movie.title, "小美 · 看电影")
        XCTAssertEqual(movie.shortDay, "3 天后")
        XCTAssertNil(MemoryCard.new(kind: .other, text: "x", when: "2026-10-07", keywords: []).monthDay(now: now))
    }

    func testUpcomingSkipsPeopleWithRemindersOff() async {
        let store = MemoryStore(directory: { nil }, backend: FakeBridge(disk: nil).backend)
        var snapshot = MemorySnapshot()
        var contact = person()
        contact.remindOn = false
        snapshot.contacts = [contact]
        snapshot.cards[contactId] = [MemoryCard.new(kind: .date, text: "生日", when: "1998-10-05", keywords: [])]
        store.replace(with: snapshot)
        XCTAssertTrue(store.upcoming(within: 6, now: MemoryDate.parse("2026-10-04")!).isEmpty, "日子提醒关了不列")
    }

    func testUpcomingIncludesEveryone() async {
        let store = MemoryStore(directory: { nil }, backend: FakeBridge(disk: nil).backend)
        var snapshot = MemorySnapshot()
        let mom = MemoryContact.new(name: "妈妈", pronoun: .ta)
        let boss = MemoryContact.new(name: "老板", pronoun: .ta)
        snapshot.contacts = [mom, boss]
        snapshot.cards[mom.id] = [MemoryCard.new(kind: .date, text: "生日", when: "1960-10-05", keywords: [])]
        snapshot.cards[boss.id] = [MemoryCard.new(kind: .promise, text: "交方案", when: "2026-10-05", keywords: [])]
        store.replace(with: snapshot)
        let items = store.upcoming(within: 6, now: MemoryDate.parse("2026-10-04")!)
        XCTAssertEqual(items.map(\.contact.name).sorted(), ["妈妈", "老板"], "名单上的人都提醒")
    }

    func testSettingsWording() {
        let mom = MemoryContact.new(name: "妈妈", pronoun: .ta)
        XCTAssertEqual(MemoryStore.Wording.switchesNote(mom), "只对妈妈生效。")
        XCTAssertEqual(MemoryStore.Wording.pinLimit(), "最多置顶 4 个人")
        XCTAssertEqual(MemoryStore.Wording.pinNote(0), "键盘上会先摆置顶的人（最多 4 个）")
        XCTAssertEqual(MemoryStore.Wording.pinNote(2), "已经置顶 2 / 4 个")
    }

    func testBadCardErrorFromBridgeIsShownAsIs() {
        let failure = MemoryFailure.decode(#"{"code":"invalid","message":"小美的卡「爱吃辣」：每个关键词要 2 到 8 个字"}"#)
        XCTAssertEqual(
            failure.map(MemoryStore.Wording.failed), "没存上：小美的卡「爱吃辣」：每个关键词要 2 到 8 个字")
    }

    // 三方合并

    func testMergeKeepsKeyboardNotesAndAppEdits() async {
        var base = MemorySnapshot()
        base.contacts = [person()]
        base.cards[contactId] = [card("a", "原来的", touched: 1), card("b", "要删的", touched: 1)]
        var local = base
        local.cards[contactId] = [card("a", "App 改的", touched: 5), card("c", "App 新加的", touched: 5)]
        var remote = base
        remote.cards[contactId] = base.cards[contactId]! + [card("k", "键盘记的", touched: 3)]
        remote.revs[contactId] = 4
        let merged = MemoryMerge.merge(base: base, local: local, remote: remote)
        XCTAssertEqual(merged.cards[contactId]?.map(\.text), ["App 改的", "键盘记的", "App 新加的"])
        XCTAssertEqual(merged.revs[contactId], 4, "修订号用重读的")
    }

    func testMergeBothChangedNewerWinsAndContactsUnion() async {
        var base = MemorySnapshot()
        base.contacts = [person()]
        base.cards[contactId] = [card("a", "原来的", touched: 1)]
        var local = base
        local.cards[contactId] = [card("a", "App 改的", touched: 2)]
        let other = MemoryContact(
            id: "11111111111111111111111111111111", name: "阿杰", pronoun: .taM,
            createdAt: 0)
        local.contacts.append(other)
        var remote = base
        remote.cards[contactId] = [card("a", "键盘那边更新的", touched: 9)]
        let merged = MemoryMerge.merge(base: base, local: local, remote: remote)
        XCTAssertEqual(merged.cards[contactId]?.first?.text, "键盘那边更新的", "两边都改了，touchedAt 新的赢")
        XCTAssertEqual(merged.contacts.map(\.name), ["小美", "阿杰"])
        var forgot = base
        forgot.contacts = []
        forgot.cards = [:]
        XCTAssertTrue(MemoryMerge.merge(base: base, local: forgot, remote: remote).contacts.isEmpty, "App 删掉的人照样删")
    }

    // 失败都要有提示

    func testMissingAppGroupIsAVisibleError() async {
        let store = MemoryStore(directory: { nil }, backend: FakeBridge(disk: sampleSnapshot()).backend)
        await store.reload()
        XCTAssertEqual(store.loadError, MemoryStore.Wording.noAppGroup, "首页常驻显示，不是静默的空列表")
        XCTAssertTrue(store.snapshot.contacts.isEmpty)
        XCTAssertFalse(store.canEdit)
        let ok1 = await store.saveCard(card("x", "新的", touched: 1), for: contactId)
        XCTAssertFalse(ok1)
        XCTAssertEqual(store.message, MemoryStore.Wording.noAppGroup, "写也要弹出原因")
    }

    func testUnreadableMemoryIsAVisibleError() async {
        let store = await store(FakeBridge(disk: nil))
        XCTAssertEqual(store.loadError, MemoryStore.Wording.unreadable)
        XCTAssertFalse(store.canEdit, "没读出来时不让写，免得空数据把文件覆盖")
        let ok2 = await store.saveCard(card("x", "新的", touched: 1), for: contactId)
        XCTAssertFalse(ok2)
        XCTAssertEqual(store.message, MemoryStore.Wording.unreadable)
    }

    func testReloadClearsLoadErrorOnceReadable() async {
        let bridge = FakeBridge(disk: nil)
        let store = await store(bridge)
        XCTAssertNotNil(store.loadError)
        bridge.disk = sampleSnapshot()
        await store.reload()
        XCTAssertNil(store.loadError)
        XCTAssertEqual(store.snapshot.contacts.map(\.name), ["小美"])
    }

    func testBrokenFileIsReported() async {
        var snapshot = sampleSnapshot()
        snapshot.broken = [contactId]
        let store = await store(FakeBridge(disk: snapshot))
        XCTAssertEqual(store.message, "小美的记忆文件坏了，已备份；坏的部分没读进来")
    }

    func testLockTimeoutKeepsDataAndTellsWhy() async {
        let bridge = FakeBridge(disk: sampleSnapshot())
        bridge.writeResults = [MemoryFailure.decode(#"{"code":"lock_timeout","message":"记忆正被另一处使用，稍后再试"}"#)]
        let store = await store(bridge)
        let ok3 = await store.saveCard(card("x", "新的", touched: 1), for: contactId)
        XCTAssertFalse(ok3)
        XCTAssertEqual(store.message, "没存上：键盘正在写记忆，请稍后再试")
        XCTAssertEqual(store.cards(of: contactId).map(\.text), ["原来的"], "失败时内存里的不改")
    }

    func testBridgeErrorsAreShownWithReason() async {
        let bridge = FakeBridge(disk: sampleSnapshot())
        bridge.writeResults = [
            MemoryFailure.decode(#"{"code":"pin_limit","message":"最多置顶 4 个人"}"#),
            MemoryFailure.decode(#"{"code":"io","message":"记忆文件读写不了（开机后还没解锁过时读不到），请解锁后重试"}"#),
            MemoryFailure.decode("不是 JSON"),
        ]
        let store = await store(bridge)
        let ok4 = await store.addContact(MemoryContact.new(name: "阿杰", pronoun: .taM), cards: [])
        XCTAssertFalse(ok4)
        XCTAssertEqual(store.message, "没存上：最多置顶 4 个人")
        let ok5 = await store.forget(contactId)
        XCTAssertFalse(ok5)
        XCTAssertEqual(store.message, "没存上：记忆文件读写不了（开机后还没解锁过时读不到），请解锁后重试")
        let ok6 = await store.deleteCard("a", for: contactId)
        XCTAssertFalse(ok6)
        XCTAssertEqual(store.message, "没存上：不是 JSON")
        XCTAssertEqual(store.snapshot.contacts.count, 1)
    }

    func testConflictMergesRereadsAndTellsTheUser() async {
        let bridge = FakeBridge(disk: sampleSnapshot())
        let store = await store(bridge)
        var remote = sampleSnapshot()
        remote.cards[contactId]?.append(card("k", "键盘记的", touched: 3))
        remote.revs[contactId] = 2
        bridge.disk = remote
        bridge.writeResults = [MemoryFailure.decode(#"{"code":"conflict","message":"x"}"#), nil]
        let ok7 = await store.saveCard(card("c", "App 新加的", touched: 5), for: contactId)
        XCTAssertTrue(ok7)
        XCTAssertEqual(bridge.writes.count, 2)
        XCTAssertEqual(bridge.writes.last?.revs[contactId], 2, "第二次带重读的修订号")
        XCTAssertEqual(store.cards(of: contactId).map(\.text), ["原来的", "键盘记的", "App 新加的"])
        XCTAssertEqual(store.message, MemoryStore.Wording.merged)
    }

    func testRepeatedConflictsGiveUpAndReloadLatest() async {
        let bridge = FakeBridge(disk: sampleSnapshot())
        let store = await store(bridge)
        var remote = sampleSnapshot()
        remote.cards[contactId]?.append(card("k", "键盘记的", touched: 3))
        bridge.disk = remote
        let conflict = MemoryFailure.decode(#"{"code":"conflict","message":"x"}"#)
        bridge.writeResults = [conflict, conflict, conflict]
        let ok8 = await store.saveCard(card("c", "App 新加的", touched: 5), for: contactId)
        XCTAssertFalse(ok8)
        XCTAssertEqual(store.message, MemoryStore.Wording.conflictGaveUp)
        XCTAssertEqual(store.cards(of: contactId).map(\.text), ["原来的", "键盘记的"], "界面换成最新的")
    }

    func testConflictWithUnreadableRereadIsReported() async {
        let bridge = FakeBridge(disk: sampleSnapshot())
        let store = await store(bridge)
        bridge.disk = nil
        bridge.writeResults = [MemoryFailure.decode(#"{"code":"conflict","message":"x"}"#)]
        let ok9 = await store.saveCard(card("c", "App 新加的", touched: 5), for: contactId)
        XCTAssertFalse(ok9)
        XCTAssertEqual(store.message, MemoryStore.Wording.conflictUnreadable)
    }

    func testFailureCodesIncludeLockTimeout() async {
        XCTAssertEqual(MemoryFailure.decode(#"{"code":"lock_timeout","message":"x"}"#)?.code, .lockTimeout)
        XCTAssertEqual(MemoryFailure.decode(#"{"code":"contact_limit","message":"恋爱最多 8 个人"}"#)?.userMessage, "恋爱最多 8 个人")
    }

    // 后台写与界面状态

    func testSecondSaveIsRejectedWhileSaving() async {
        let bridge = FakeBridge(disk: sampleSnapshot())
        let store = await store(bridge)
        let gate = DispatchSemaphore(value: 0)
        bridge.gate = gate
        let first = Task { await store.saveCard(card("c", "第一笔", touched: 5), for: contactId) }
        while !store.saving { await Task.yield() }
        let second = await store.saveCard(card("d", "第二笔", touched: 6), for: contactId)
        XCTAssertFalse(second, "保存中再提交直接拒掉")
        XCTAssertTrue(store.saving)
        gate.signal()
        let firstSaved = await first.value
        XCTAssertTrue(firstSaved)
        XCTAssertFalse(store.saving)
        XCTAssertEqual(bridge.writes.count, 1, "只写了一次")
        XCTAssertEqual(store.cards(of: contactId).map(\.text), ["原来的", "第一笔"])
    }

    func testBackgroundFailureComesBackAsMessageOnMain() async {
        let bridge = FakeBridge(disk: sampleSnapshot())
        bridge.writeResults = [MemoryFailure.decode(#"{"code":"lock_timeout","message":"x"}"#)]
        let store = await store(bridge)
        let saved = await store.saveCard(card("c", "新的", touched: 5), for: contactId)
        XCTAssertFalse(saved)
        XCTAssertFalse(bridge.wroteOnMain, "写在后台队列上")
        XCTAssertTrue(Thread.isMainThread, "结果回到主线程")
        XCTAssertEqual(store.message, "没存上：键盘正在写记忆，请稍后再试")
        XCTAssertFalse(store.saving)
    }

    func testConcurrentReloadsReadOnce() async {
        let bridge = FakeBridge(disk: sampleSnapshot())
        let store = MemoryStore(directory: { [directory] in directory }, backend: bridge.backend)
        async let first: Void = store.reload()
        async let second: Void = store.reload()
        _ = await (first, second)
        XCTAssertEqual(bridge.reads, 1, "启动时 .task 与回到前台连着读，只读一次")
        XCTAssertTrue(store.loaded)
    }

    func testForgetHalfDoneSaysCardsAreGone() async {
        let bridge = FakeBridge(disk: sampleSnapshot())
        let store = await store(bridge)
        var halfDone = sampleSnapshot()
        halfDone.cards[contactId] = nil
        bridge.disk = halfDone
        bridge.writeResults = [MemoryFailure.decode(#"{"code":"io","message":"x"}"#)]
        let forgot = await store.forget(contactId)
        XCTAssertFalse(forgot)
        XCTAssertEqual(store.message, "小美的卡片已经删了，但名单没更新上，请再点一次「忘掉这个人」")
        XCTAssertTrue(store.cards(of: contactId).isEmpty, "界面换成重读的")
    }

    func testBrokenFileFoundWhileMergingIsReported() async {
        let bridge = FakeBridge(disk: sampleSnapshot())
        let store = await store(bridge)
        var remote = sampleSnapshot()
        remote.broken = [contactId]
        remote.cards[contactId] = []
        bridge.disk = remote
        bridge.writeResults = [MemoryFailure.decode(#"{"code":"conflict","message":"x"}"#), nil]
        let saved = await store.saveCard(card("c", "App 新加的", touched: 5), for: contactId)
        XCTAssertTrue(saved)
        XCTAssertEqual(
            store.message, "记忆刚有更新，已经和你的修改合在一起存好了\n小美的记忆文件坏了，已备份；坏的部分没读进来")
    }

    func testRemindOffNoteUsesPronoun() {
        var contact = person()
        contact.pronoun = .taF
        XCTAssertEqual(MemoryStore.Wording.remindOffNote(contact), "关掉后，今天和本周里不再提她的日子")
        contact.pronoun = .taM
        XCTAssertEqual(MemoryStore.Wording.remindOffNote(contact), "关掉后，今天和本周里不再提他的日子")
        contact.pronoun = .name
        XCTAssertEqual(MemoryStore.Wording.remindOffNote(contact), "关掉后，今天和本周里不再提小美的日子")
    }

    // 数据保护

    /// 用 complete 的话锁屏时键盘读不到卡片，锁屏通知里回复时提示行失效。
    func testMemoryDirectoryProtectionLetsKeyboardReadWhileLocked() async throws {
        XCTAssertEqual(MemoryStore.protection, .completeUntilFirstUserAuthentication)
        let memory = directory.appendingPathComponent("protect-\(UUID().uuidString)/memory", isDirectory: true)
        MemoryStore.protect(memory)
        var isDirectory: ObjCBool = false
        XCTAssertTrue(FileManager.default.fileExists(atPath: memory.path, isDirectory: &isDirectory) && isDirectory.boolValue)
        // 模拟器不一定回报保护级别；回报了就必须是这一档
        let attributes = try FileManager.default.attributesOfItem(atPath: memory.path)
        if let level = attributes[.protectionKey] as? FileProtectionType {
            XCTAssertEqual(level, .completeUntilFirstUserAuthentication)
        }
    }

    // 导出

    func testExportTextGroupsByKind() async {
        let store = MemoryStore(directory: { nil }, backend: FakeBridge(disk: nil).backend)
        var snapshot = MemorySnapshot()
        snapshot.contacts = [person()]
        snapshot.cards[contactId] = [
            MemoryCard.new(kind: .preference, text: "不吃香菜", when: nil, keywords: []),
            MemoryCard.new(kind: .date, text: "生日", when: "1998-10-05", keywords: []),
        ]
        store.replace(with: snapshot)
        let lines = store.exportText(contactId, now: MemoryDate.parse("2026-10-04")!).components(separatedBy: "\n")
        XCTAssertTrue(lines[0].hasPrefix("小美（认识 "))
        XCTAssertEqual(Array(lines.dropFirst()), ["", "日子", "· 生日 1998-10-05", "", "喜好", "· 不吃香菜"])
    }
}
