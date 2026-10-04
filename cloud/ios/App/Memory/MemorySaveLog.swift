// 每次保存记一条系统日志（subsystem app.qingjian.cloud，category memory），给真机上查并发冲突用：
// 序号、写入前后这个人的修订号、写之前锁是不是被别人占着、写入总耗时（含等锁）、冲突合并了几轮、成败。不记名字与卡片正文。
// 桥不报告等锁时长，所以「等了多久」用 qj_memory_write 调用的总耗时近似。

import os

struct MemorySaveLog {
    private static let logger = Logger(subsystem: "app.qingjian.cloud", category: "memory")

    var seq: Int

    /// 写入前这个人的修订号；新建的人没有，记 -1。
    var revBefore: Int64 = -1

    /// 写完重读到的修订号；失败或人已删掉记 -1。
    var revAfter: Int64 = -1

    /// 第一次写之前探到 memory/.lock 被别的进程占着。
    var lockBusy = false

    /// 所有 qj_memory_write 调用加起来的毫秒数，含桥里等锁的时间。
    var writeMillis = 0

    /// 遇到 conflict 后合并重写的轮数。
    var conflicts = 0

    var outcome = "pending"

    func emit() {
        Self.logger.notice(
            """
            save #\(seq, privacy: .public) rev \(revBefore, privacy: .public)->\(revAfter, privacy: .public) \
            lockBusy=\(lockBusy, privacy: .public) writeMs=\(writeMillis, privacy: .public) \
            conflicts=\(conflicts, privacy: .public) outcome=\(outcome, privacy: .public)
            """)
    }
}
