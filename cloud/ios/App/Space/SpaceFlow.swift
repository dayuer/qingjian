// 输码之后等旧设备允许：多久问一次、什么时候停。到 `expires_at` 就停，不再问——那张申请在服务端已经不作数了。

import Foundation

struct SpaceFlow: Equatable {
    /// 每两次轮询之间等多久。
    static let interval: TimeInterval = 2

    /// 这次申请的过期时间，Unix 毫秒（桥给的 `expires_at`）。
    let expiresAt: Int64

    func shouldPoll(now: Int64) -> Bool {
        now < expiresAt
    }

    /// 下一次轮询前等多久：不会越过 `expires_at`，也不会是 0。
    func nextDelay(now: Int64) -> TimeInterval {
        let left = TimeInterval(expiresAt - now) / 1000
        return max(0.2, min(Self.interval, left))
    }
}
