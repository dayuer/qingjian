// App 侧「键盘记住的事」的数据层：本周挑卡、写回冲突的三方合并、导出文本，以及每条写入失败路径（容器拿不到、
// 读不出、锁超时、冲突、桥报错）都落成界面上能显示的中文，不静默。桥用假的 MemoryBackend 代替。

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
        MemoryContact(id: contactId, name: name, pronoun: .ta, scene: MemoryScope.dating, createdAt: 0)
    }

    private func sampleSnapshot() -> MemorySnapshot {
        var snapshot = MemorySnapshot()
        snapshot.contacts = [person()]
        snapshot.cards[contactId] = [card("a", "原来的", touched: 1)]
        snapshot.revs[contactId] = 1
        return snapshot
    }

    /// 假桥：读返回 `disk`，写按 `writeResults` 依次给结果（用完后都算成功），成功时把写的内容落到 `disk`。
    private final class FakeBridge {
        var disk: MemorySnapshot?

        var writeResults: [MemoryFailure?] = []

        var writes: [MemorySnapshot] = []

        init(disk: MemorySnapshot?) { self.disk = disk }

        var backend: MemoryBackend {
            MemoryBackend(
                read: { _ in self.disk },
                write: { snapshot, _ in
                    self.writes.append(snapshot)
                    let result = self.writeResults.isEmpty ? nil : self.writeResults.removeFirst()
                    if result == nil { self.disk = snapshot }
                    return result
                })
        }
    }

    private func store(_ bridge: FakeBridge, directory: URL? = nil) -> MemoryStore {
        let dir = directory ?? self.directory
        let store = MemoryStore(directory: { dir }, backend: bridge.backend)
        store.reload()
        return store
    }

    // 本周与今天

    func testUpcomingPicksDatesWithinRange() {
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

    func testUpcomingCardTitles() {
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

    func testUpcomingSkipsPeopleWithRemindersOff() {
        let store = MemoryStore(directory: { nil }, backend: FakeBridge(disk: nil).backend)
        var snapshot = MemorySnapshot()
        var contact = person()
        contact.remindOn = false
        snapshot.contacts = [contact]
        snapshot.cards[contactId] = [MemoryCard.new(kind: .date, text: "生日", when: "1998-10-05", keywords: [])]
        store.replace(with: snapshot)
        XCTAssertTrue(store.upcoming(within: 6, now: MemoryDate.parse("2026-10-04")!).isEmpty, "日子提醒关了不列")
    }

    // 三方合并

    func testMergeKeepsKeyboardNotesAndAppEdits() {
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

    func testMergeBothChangedNewerWinsAndContactsUnion() {
        var base = MemorySnapshot()
        base.contacts = [person()]
        base.cards[contactId] = [card("a", "原来的", touched: 1)]
        var local = base
        local.cards[contactId] = [card("a", "App 改的", touched: 2)]
        let other = MemoryContact(
            id: "11111111111111111111111111111111", name: "阿杰", pronoun: .taM, scene: MemoryScope.dating,
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

    func testMissingAppGroupIsAVisibleError() {
        let store = MemoryStore(directory: { nil }, backend: FakeBridge(disk: sampleSnapshot()).backend)
        store.reload()
        XCTAssertEqual(store.loadError, MemoryStore.Text.noAppGroup, "首页常驻显示，不是静默的空列表")
        XCTAssertTrue(store.people.isEmpty)
        XCTAssertFalse(store.canEdit)
        XCTAssertFalse(store.saveCard(card("x", "新的", touched: 1), for: contactId))
        XCTAssertEqual(store.message, MemoryStore.Text.noAppGroup, "写也要弹出原因")
    }

    func testUnreadableMemoryIsAVisibleError() {
        let store = store(FakeBridge(disk: nil))
        XCTAssertEqual(store.loadError, MemoryStore.Text.unreadable)
        XCTAssertFalse(store.canEdit, "没读出来时不让写，免得空数据把文件覆盖")
        XCTAssertFalse(store.saveCard(card("x", "新的", touched: 1), for: contactId))
        XCTAssertEqual(store.message, MemoryStore.Text.unreadable)
    }

    func testReloadClearsLoadErrorOnceReadable() {
        let bridge = FakeBridge(disk: nil)
        let store = store(bridge)
        XCTAssertNotNil(store.loadError)
        bridge.disk = sampleSnapshot()
        store.reload()
        XCTAssertNil(store.loadError)
        XCTAssertEqual(store.people.map(\.name), ["小美"])
    }

    func testBrokenFileIsReported() {
        var snapshot = sampleSnapshot()
        snapshot.broken = [contactId]
        let store = store(FakeBridge(disk: snapshot))
        XCTAssertEqual(store.message, "小美的记忆文件坏了，已备份；坏的部分没读进来")
    }

    func testLockTimeoutKeepsDataAndTellsWhy() {
        let bridge = FakeBridge(disk: sampleSnapshot())
        bridge.writeResults = [MemoryFailure.decode(#"{"code":"lock_timeout","message":"记忆正被另一处使用，稍后再试"}"#)]
        let store = store(bridge)
        XCTAssertFalse(store.saveCard(card("x", "新的", touched: 1), for: contactId))
        XCTAssertEqual(store.message, "没存上：键盘正在写记忆，请稍后再试")
        XCTAssertEqual(store.cards(of: contactId).map(\.text), ["原来的"], "失败时内存里的不改")
    }

    func testBridgeErrorsAreShownWithReason() {
        let bridge = FakeBridge(disk: sampleSnapshot())
        bridge.writeResults = [
            MemoryFailure.decode(#"{"code":"contact_limit","message":"x"}"#),
            MemoryFailure.decode(#"{"code":"io","message":"记忆文件读写不了（锁屏时读不到），请解锁后重试"}"#),
            MemoryFailure.decode("不是 JSON"),
        ]
        let store = store(bridge)
        XCTAssertFalse(store.addContact(MemoryContact.new(name: "阿杰", pronoun: .taM), cards: []))
        XCTAssertEqual(store.message, "没存上：恋爱场景最多 8 个人")
        XCTAssertFalse(store.forget(contactId))
        XCTAssertEqual(store.message, "没存上：记忆文件读写不了（锁屏时读不到），请解锁后重试")
        XCTAssertFalse(store.deleteCard("a", for: contactId))
        XCTAssertEqual(store.message, "没存上：不是 JSON")
        XCTAssertEqual(store.people.count, 1)
    }

    func testConflictMergesRereadsAndTellsTheUser() {
        let bridge = FakeBridge(disk: sampleSnapshot())
        let store = store(bridge)
        var remote = sampleSnapshot()
        remote.cards[contactId]?.append(card("k", "键盘记的", touched: 3))
        remote.revs[contactId] = 2
        bridge.disk = remote
        bridge.writeResults = [MemoryFailure.decode(#"{"code":"conflict","message":"x"}"#), nil]
        XCTAssertTrue(store.saveCard(card("c", "App 新加的", touched: 5), for: contactId))
        XCTAssertEqual(bridge.writes.count, 2)
        XCTAssertEqual(bridge.writes.last?.revs[contactId], 2, "第二次带重读的修订号")
        XCTAssertEqual(store.cards(of: contactId).map(\.text), ["原来的", "键盘记的", "App 新加的"])
        XCTAssertEqual(store.message, MemoryStore.Text.merged)
    }

    func testRepeatedConflictsGiveUpAndReloadLatest() {
        let bridge = FakeBridge(disk: sampleSnapshot())
        let store = store(bridge)
        var remote = sampleSnapshot()
        remote.cards[contactId]?.append(card("k", "键盘记的", touched: 3))
        bridge.disk = remote
        let conflict = MemoryFailure.decode(#"{"code":"conflict","message":"x"}"#)
        bridge.writeResults = [conflict, conflict, conflict]
        XCTAssertFalse(store.saveCard(card("c", "App 新加的", touched: 5), for: contactId))
        XCTAssertEqual(store.message, MemoryStore.Text.conflictGaveUp)
        XCTAssertEqual(store.cards(of: contactId).map(\.text), ["原来的", "键盘记的"], "界面换成最新的")
    }

    func testConflictWithUnreadableRereadIsReported() {
        let bridge = FakeBridge(disk: sampleSnapshot())
        let store = store(bridge)
        bridge.disk = nil
        bridge.writeResults = [MemoryFailure.decode(#"{"code":"conflict","message":"x"}"#)]
        XCTAssertFalse(store.saveCard(card("c", "App 新加的", touched: 5), for: contactId))
        XCTAssertEqual(store.message, MemoryStore.Text.conflictUnreadable)
    }

    func testFailureCodesIncludeLockTimeout() {
        XCTAssertEqual(MemoryFailure.decode(#"{"code":"lock_timeout","message":"x"}"#)?.code, .lockTimeout)
        XCTAssertEqual(MemoryFailure.decode(#"{"code":"contact_limit","message":"x"}"#)?.userMessage, "恋爱场景最多 8 个人")
    }

    // 数据保护

    /// 用 complete 的话锁屏时键盘读不到卡片，锁屏通知里回复时提示行失效。
    func testMemoryDirectoryProtectionLetsKeyboardReadWhileLocked() throws {
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

    func testExportTextGroupsByKind() {
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
