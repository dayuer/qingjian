// 输码前的本地判定：不够 8 位就不碰桥。

import XCTest
@testable import QingjianCloud

@MainActor
final class SpaceStoreTests: XCTestCase {
    func testAnIncompleteCodeIsRejectedBeforeAnyBridgeCall() async {
        let store = SpaceStore()
        var calls = 0
        store.fileProvider = {
            calls += 1
            return nil
        }
        await store.join(code: "K7P2-9")
        XCTAssertEqual(store.message, SpaceWording.missingCode)
        XCTAssertFalse(store.waiting)
        XCTAssertEqual(calls, 0, "不够 8 位不该碰桥")
    }

    func testNoAppGroupIsSaidPlainlyAndCountsAsNotOpened() {
        let store = SpaceStore()
        store.fileProvider = { nil }
        store.refresh()
        XCTAssertEqual(store.signedIn, false)
        XCTAssertEqual(store.message, SpaceStore.noGroup)
    }
}
