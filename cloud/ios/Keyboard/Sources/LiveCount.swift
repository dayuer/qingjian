// 真机查泄漏用：引擎、键盘模型、控制器在 init / deinit 时报一次存活个数，写进系统日志（`sujian.keyboard` / `perf`）。
// 只在 `Diagnostics.enabled` 时写。
// 键盘每次出现都会新建控制器与引擎，旧的不析构就是泄漏。deinit 不在主线程隔离里，计数用锁。

import Foundation
import os

enum LiveCount {
    private static let log = Logger(subsystem: "sujian.keyboard", category: "perf")
    private static let lock = OSAllocatedUnfairLock(initialState: [String: Int]())

    static func created(_ kind: String) { change(kind, +1) }

    static func note(_ text: String) {
        guard Diagnostics.enabled else { return }
        log.info("\(text, privacy: .public)")
    }

    static func released(_ kind: String) { change(kind, -1) }

    private static func change(_ kind: String, _ delta: Int) {
        guard Diagnostics.enabled else { return }
        let alive = lock.withLock { counts in
            counts[kind, default: 0] += delta
            return counts[kind, default: 0]
        }
        log.info("live=\(kind, privacy: .public) count=\(alive, privacy: .public) delta=\(delta, privacy: .public)")
    }
}
