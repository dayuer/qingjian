// 账号页状态的并发保护、邮箱裁剪，以及换引擎后的显示。

import Foundation
import XCTest
@testable import QingjianCloud

private final class Counter: @unchecked Sendable {
    private let lock = NSLock()

    private var value = 0

    func bump() { lock.lock(); value += 1; lock.unlock() }

    var count: Int { lock.lock(); defer { lock.unlock() }; return value }
}

@MainActor
final class AccountStoreTests: XCTestCase {
    func testMayEnter() {
        XCTAssertTrue(AccountStore.mayEnter(busy: false))
        XCTAssertFalse(AccountStore.mayEnter(busy: true))
    }

    func testRepeatedPerformRunsWorkOnce() async {
        let dir = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try? FileManager.default.createDirectory(at: dir, withIntermediateDirectories: true)
        let store = AccountStore()
        store.fileProvider = { dir.appendingPathComponent("cloud.toml") }
        let counter = Counter()
        let work: @Sendable (URL) -> AccountFailure? = { _ in
            counter.bump()
            Thread.sleep(forTimeInterval: 0.2)
            return nil
        }
        async let first = store.perform(work, refreshAfter: false)
        async let second = store.perform(work, refreshAfter: false)
        let results = await [first, second]
        XCTAssertEqual(results.filter { $0 }.count, 1)
        XCTAssertEqual(counter.count, 1)
        XCTAssertFalse(store.busy)
    }

    func testEmailIsTrimmed() {
        XCTAssertEqual(AccountStore.normalized(email: "  a@b.c \n"), "a@b.c")
    }
}

final class EngineDisplayTests: XCTestCase {
    func testNoEngineClearsDisplay() {
        let old = [CandidateItem(text: "你", cloud: false)]
        let cleared = EngineDisplay.afterReplace(hasEngine: false, preedit: "ni", candidates: old)
        XCTAssertEqual(cleared.preedit, "")
        XCTAssertTrue(cleared.candidates.isEmpty)
        let kept = EngineDisplay.afterReplace(hasEngine: true, preedit: "ni", candidates: old)
        XCTAssertEqual(kept.preedit, "ni")
        XCTAssertEqual(kept.candidates, old)
    }
}
