// 等旧设备允许的那个循环：多久问一次、什么时候停。

import XCTest
@testable import QingjianCloud

final class SpaceFlowTests: XCTestCase {
    private let now: Int64 = 1_791_043_200_000

    func testItPollsEveryTwoSecondsWhileTheTicketIsAlive() {
        let flow = SpaceFlow(expiresAt: now + 60_000)
        XCTAssertTrue(flow.shouldPoll(now: now))
        XCTAssertEqual(flow.nextDelay(now: now), 2.0, accuracy: 0.001)
    }

    func testItStopsWhenTheTicketExpires() {
        let flow = SpaceFlow(expiresAt: now + 60_000)
        XCTAssertFalse(flow.shouldPoll(now: now + 60_000), "到点就停，不再问")
        XCTAssertFalse(flow.shouldPoll(now: now + 60_001))
    }

    func testAnAlreadyExpiredTicketNeverPolls() {
        let flow = SpaceFlow(expiresAt: now)
        XCTAssertFalse(flow.shouldPoll(now: now))
    }

    func testTheLastPollBeforeExpiryWaitsShorterRatherThanOverrunning() {
        // 还剩 0.5 秒：等 0.5 秒就问最后一次取到结果，而不是干等 2 秒、过了期还不知道
        let flow = SpaceFlow(expiresAt: now + 500)
        XCTAssertEqual(flow.nextDelay(now: now), 0.5, accuracy: 0.001)
    }

    func testTheDelayIsNeverZeroOrNegative() {
        let flow = SpaceFlow(expiresAt: now + 1)
        XCTAssertGreaterThan(flow.nextDelay(now: now), 0)
    }
}
