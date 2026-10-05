// 对象详情「待整理」：桥返回的解码、每条的时间与来源、收起只显示前 2 行、180 条提示同一个人只出一次，
// 以及读删走后台时的失败路径（读不出常驻原因、删失败弹「没删掉：」、删成功后重读）。桥用假的 MaterialsBackend 代替。
// 也锁住 conflict 的统一说法「记忆刚有更新」。

import Foundation
import XCTest
@testable import QingjianCloud

@MainActor
final class MaterialsTests: XCTestCase {
    private let contactId = "0123456789abcdef0123456789abcdef"

    private let directory = URL(fileURLWithPath: NSTemporaryDirectory(), isDirectory: true)

    /// 2026-10-05 10:00 北京时间。
    private let now = Date(timeIntervalSince1970: 1_791_165_600)

    private func material(_ id: String, _ text: String, at: Int64 = 1_791_165_000) -> MemoryMaterial {
        MemoryMaterial(clientId: id, text: text, at: at, source: .clipboard)
    }

    // MARK: 解码

    func testDecodesBridgeListing() throws {
        let json = #"""
        {"unprocessed_count":2,"materials":[
          {"client_id":"b","kind":"note","text":"小美 09:34\n周六去看展","at":1791165000,"source":"clipboard","uploaded":false,"processed":false},
          {"client_id":"a","kind":"note","text":"她怕冷","at":1791100000,"source":"zzz","uploaded":false,"processed":false}]}
        """#
        let list: MaterialList = try XCTUnwrap(MemoryFiles.decode(json))
        XCTAssertEqual(list.unprocessedCount, 2)
        XCTAssertEqual(list.materials.map(\.id), ["b", "a"])
        XCTAssertEqual(list.materials[0].text, "小美 09:34\n周六去看展", "原话原样，名字和时间行都在")
        XCTAssertEqual(list.materials[0].source, .clipboard)
        XCTAssertEqual(list.materials[1].source, .typed, "认不得的来源按手写")
    }

    // MARK: 格式化

    func testTitleAndMeta() {
        XCTAssertEqual(MaterialDisplay.title(count: 3), "待整理 · 3 条")
        XCTAssertEqual(MaterialDisplay.meta(at: 1_791_165_000, source: .clipboard, now: now), "今天 09:50 · 剪贴板")
        XCTAssertEqual(MaterialDisplay.meta(at: 1_791_165_000 - 86_400, source: .typed, now: now), "昨天 09:50 · 手写")
        XCTAssertEqual(MaterialDisplay.time(at: 1_791_165_000 - 3 * 86_400, now: now), "10月2日 09:50")
        XCTAssertEqual(MaterialDisplay.time(at: 1_760_000_000, now: now), "2025年10月9日 16:53")
    }

    func testCollapsedShowsFirstTwoLines() {
        let wechat = "小美 2026年10月05日 09:34\n周六想去看那个展\n我 2026年10月05日 09:35\n好呀"
        XCTAssertEqual(MaterialDisplay.preview(wechat), "小美 2026年10月05日 09:34\n周六想去看那个展")
        XCTAssertTrue(MaterialDisplay.isLong(wechat))
        XCTAssertEqual(MaterialDisplay.preview("一行\n两行"), "一行\n两行")
        XCTAssertFalse(MaterialDisplay.isLong("一行\n两行"))
        XCTAssertFalse(MaterialDisplay.isLong("她怕冷"))
        let long = String(repeating: "字", count: 60)
        XCTAssertEqual(MaterialDisplay.preview(long), long, "单行不截，折行交给 lineLimit")
        XCTAssertTrue(MaterialDisplay.isLong(long), "一行太长折成三行也算看不全")
    }

    func testCloudHintAndDeleteTexts() {
        XCTAssertEqual(MaterialDisplay.cloudHint, "开通素笺云后，每天帮你整理成记忆卡")
        XCTAssertEqual(MaterialDisplay.deleteTitle, "删掉这条原话？")
    }

    // MARK: 180 条提示

    func testNudgeThreshold() {
        XCTAssertFalse(MaterialNudge.shouldShow(count: 179, alreadyShown: false))
        XCTAssertTrue(MaterialNudge.shouldShow(count: 180, alreadyShown: false))
        XCTAssertTrue(MaterialNudge.shouldShow(count: 200, alreadyShown: false))
        XCTAssertFalse(MaterialNudge.shouldShow(count: 190, alreadyShown: true))
        XCTAssertEqual(MaterialNudge.text(count: 183), "待整理快满了（183 / 200），开通素笺云整理，或者删掉一些")
    }

    func testNudgeShowsOncePerContact() throws {
        let suite = "materials-nudge-tests"
        let defaults = try XCTUnwrap(UserDefaults(suiteName: suite))
        defaults.removePersistentDomain(forName: suite)
        defer { defaults.removePersistentDomain(forName: suite) }
        let nudge = MaterialNudge(defaults: defaults)
        XCTAssertNil(nudge.take(contactId: contactId, count: 179), "不到 180 不提示，也不记")
        XCTAssertEqual(nudge.take(contactId: contactId, count: 180), MaterialNudge.text(count: 180))
        XCTAssertNil(nudge.take(contactId: contactId, count: 181), "同一个人提示过就不再提示")
        XCTAssertNil(MaterialNudge(defaults: defaults).take(contactId: contactId, count: 200), "重开 App 也记得")
        XCTAssertNotNil(nudge.take(contactId: "other", count: 190), "别的人各算各的")
        XCTAssertEqual(defaults.stringArray(forKey: MaterialNudge.key), [contactId, "other"].sorted(), "只记对象 id")
    }

    // MARK: 读删

    /// 假桥：读按 `lists` 依次给（用完后给最后一个），删按 `deleteResult` 给；记下删了哪条。
    private final class FakeBridge: @unchecked Sendable {
        private let lock = NSLock()

        var lists: [MaterialList?] = []

        var deleteResult: MemoryFailure?

        private(set) var deleted: [String] = []

        func next() -> MaterialList? {
            lock.lock()
            defer { lock.unlock() }
            return lists.count > 1 ? lists.removeFirst() : lists.first ?? nil
        }

        func delete(_ id: String) -> MemoryFailure? {
            lock.lock()
            defer { lock.unlock() }
            deleted.append(id)
            return deleteResult
        }

        var backend: MaterialsBackend {
            MaterialsBackend(list: { _, _ in self.next() }, delete: { _, _, id in self.delete(id) })
        }
    }

    func testUnreadableShowsReasonAndRetryRecovers() async {
        let bridge = FakeBridge()
        let full = MaterialList(unprocessedCount: 1, materials: [material("a", "她怕冷")])
        bridge.lists = [nil, full]
        let store = MaterialsStore(directory: { self.directory }, backend: bridge.backend)
        await store.reload(contactId)
        XCTAssertEqual(store.loadError, MaterialsStore.Wording.unreadable)
        XCTAssertNil(store.list)
        await store.reload(contactId)
        XCTAssertNil(store.loadError)
        XCTAssertEqual(store.count, 1)
    }

    func testNoAppGroupIsNotSilent() async {
        let store = MaterialsStore(directory: { nil }, backend: FakeBridge().backend)
        await store.reload(contactId)
        XCTAssertEqual(store.loadError, MemoryStore.Wording.noAppGroup)
        let deleted = await store.delete(material("a", "x"), contactId: contactId)
        XCTAssertFalse(deleted)
        XCTAssertEqual(store.message, MemoryStore.Wording.noAppGroup)
    }

    func testDeleteFailureKeepsListAndSaysWhy() async {
        let bridge = FakeBridge()
        let list = MaterialList(unprocessedCount: 1, materials: [material("a", "她怕冷")])
        bridge.lists = [list]
        bridge.deleteResult = MemoryFailure(code: .lockTimeout, message: "x")
        let store = MaterialsStore(directory: { self.directory }, backend: bridge.backend)
        await store.reload(contactId)
        let deleted = await store.delete(list.materials[0], contactId: contactId)
        XCTAssertFalse(deleted)
        XCTAssertEqual(store.message, "没删掉：键盘正在写记忆，请稍后再试")
        XCTAssertEqual(store.list, list)
        XCTAssertNil(store.deleting)
    }

    func testDeleteSuccessRereads() async {
        let bridge = FakeBridge()
        let before = MaterialList(unprocessedCount: 2, materials: [material("b", "二"), material("a", "一")])
        let after = MaterialList(unprocessedCount: 1, materials: [material("a", "一")])
        bridge.lists = [before, after]
        let store = MaterialsStore(directory: { self.directory }, backend: bridge.backend)
        await store.reload(contactId)
        let deleted = await store.delete(before.materials[0], contactId: contactId)
        XCTAssertTrue(deleted)
        XCTAssertEqual(bridge.deleted, ["b"])
        XCTAssertEqual(store.count, 1)
        XCTAssertNil(store.message)
    }

    // MARK: conflict 的说法

    func testConflictWordingSaysMemoryJustChanged() {
        XCTAssertEqual(MemoryStore.Wording.merged, "记忆刚有更新，已经和你的修改合在一起存好了")
        XCTAssertEqual(MemoryStore.Wording.conflictGaveUp, "没存上：记忆刚有更新，已换成最新的内容，请再点一次")
        XCTAssertEqual(MemoryStore.Wording.conflictUnreadable, "没存上：记忆刚有更新，重新读取也失败了，请稍后再点一次")
        let bridge = MemoryFailure.decode(#"{"code":"conflict","message":"记忆刚有更新，请再点一次"}"#)
        XCTAssertEqual(MemoryStore.Wording.failed(bridge!), "没存上：记忆刚有更新，请再点一次")
    }
}
