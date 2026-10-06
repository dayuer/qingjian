// 改写「没成功」的两种说法分得开：网络问题说检查网络，模型给的不合用说换一个技能；等待与结果不占改写条那句话。

import XCTest
@testable import QingjianCloud

final class RewriteWordingTests: XCTestCase {
    func testTheTwoFailuresReadDifferently() {
        XCTAssertEqual(RewriteWording.note(for: .failed), RewriteWording.failed)
        XCTAssertEqual(RewriteWording.note(for: .rejected), RewriteWording.rejected)
        XCTAssertNotEqual(
            RewriteWording.failed, RewriteWording.rejected,
            "两种原因的下一步不一样，不能共用一句话")
        XCTAssertFalse(RewriteWording.failed.isEmpty)
        XCTAssertFalse(RewriteWording.rejected.isEmpty)
    }

    func testStatesWithSomethingElseToShowSayNothing() {
        XCTAssertNil(RewriteWording.note(for: .idle))
        XCTAssertNil(RewriteWording.note(for: .pending(span: RewriteSpan(original: "原话", tail: "", isSelection: false), skill: "润色")))
        XCTAssertNil(RewriteWording.note(for: .ready(span: RewriteSpan(original: "原话", tail: "", isSelection: false), skill: "润色", result: "改好的")))
    }
}
