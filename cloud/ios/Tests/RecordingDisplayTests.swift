// 「记录中 / 已暂停」标记与暂停提示条：桥给的状态怎么显示、什么时候不出。

import XCTest
@testable import QingjianCloud

final class RecordingDisplayTests: XCTestCase {
    func testBadge() {
        XCTAssertNil(RecordingDisplay.badge(state: 0, privateField: false, fullAccess: true))
        XCTAssertEqual(RecordingDisplay.badge(state: 1, privateField: false, fullAccess: true), .recording)
        XCTAssertEqual(RecordingDisplay.badge(state: 2, privateField: false, fullAccess: true), .paused)
        XCTAssertEqual(RecordingDisplay.badge(state: 3, privateField: false, fullAccess: true), .paused)
        XCTAssertNil(RecordingDisplay.badge(state: 1, privateField: true, fullAccess: true), "密码框里不出")
        XCTAssertNil(
            RecordingDisplay.badge(state: 1, privateField: false, fullAccess: false),
            "没完全访问时键盘不联网，不出")
    }

    func testCopy() {
        XCTAssertEqual(RecordingDisplay.Badge.recording.title, "记录中")
        XCTAssertEqual(RecordingDisplay.Badge.paused.title, "已暂停")
        XCTAssertEqual(RecordingDisplay.pausedBanner, "已暂停记录，提示照常。1 小时后恢复")
        XCTAssertEqual(RecordingDisplay.pauseForever, "一直暂停")
        XCTAssertEqual(RecordingDisplay.pausedForeverBanner, "已暂停记录，点「已暂停」恢复")
        XCTAssertEqual(RecordingDisplay.resumedBanner, "已恢复记录")
        XCTAssertEqual(RecordingDisplay.pauseSeconds, 3600)
    }
}
