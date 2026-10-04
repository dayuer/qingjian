// 记忆的读写都在这里做，不占主线程：桥等 memory/.lock 最多 2 秒，放在主线程会卡住界面。
// actor 跑在自己的串行队列上（自定义执行器），同一时刻只有一个读或写在跑；桥的调用是同步阻塞的，放进协作线程池会占住线程，所以不用缺省执行器。

import Darwin
import Dispatch
import Foundation

actor MemoryWorker {
    private let queue = DispatchSerialQueue(label: "app.qingjian.cloud.memory")

    private let backend: MemoryBackend

    private var saves = 0

    nonisolated var unownedExecutor: UnownedSerialExecutor { queue.asUnownedSerialExecutor() }

    init(backend: MemoryBackend) {
        self.backend = backend
    }

    func read(_ directory: URL) -> MemorySnapshot? {
        MemoryStore.protect(directory.appendingPathComponent("memory", isDirectory: true))
        return backend.read(directory)
    }

    /// 写 `next`；conflict 时重读、三方合并后再写，最多三轮。写成功后重读一遍拿新的修订号。`contactId` 只给日志用。
    /// `forgetting` 是这次要忘掉的人：写失败后重读，若那人还在名单上但卡片已经没了，就报 `forgetPartial`。
    func save(
        base: MemorySnapshot, next: MemorySnapshot, directory: URL, contactId: String?, forgetting: String? = nil
    ) -> MemorySaveResult {
        saves += 1
        var log = MemorySaveLog(seq: saves)
        log.revBefore = contactId.flatMap { base.revs[$0] }.map(Int64.init) ?? -1
        log.lockBusy = Self.lockBusy(directory)
        var result = attempt(base: base, next: next, directory: directory, log: &log)
        if case .failed = result, let id = forgetting, !(base.cards[id] ?? []).isEmpty,
           let latest = backend.read(directory), latest.contacts.contains(where: { $0.id == id }),
           (latest.cards[id] ?? []).isEmpty {
            log.outcome = "forget_partial"
            result = .forgetPartial(latest: latest)
        }
        if case .saved(_, let latest, _, _) = result {
            log.revAfter = contactId.flatMap { latest?.revs[$0] }.map(Int64.init) ?? -1
        }
        log.emit()
        return result
    }

    private func attempt(
        base: MemorySnapshot, next: MemorySnapshot, directory: URL, log: inout MemorySaveLog
    ) -> MemorySaveResult {
        var base = base
        var next = next
        var broken: [String] = []
        for _ in 0..<3 {
            next.broken = []
            let started = DispatchTime.now()
            let failure = backend.write(next, directory)
            log.writeMillis += Int((DispatchTime.now().uptimeNanoseconds - started.uptimeNanoseconds) / 1_000_000)
            guard let failure else {
                log.outcome = "saved"
                return .saved(
                    written: next, latest: backend.read(directory), merged: log.conflicts > 0, brokenOnMerge: broken)
            }
            guard failure.code == .conflict else {
                log.outcome = failure.code.rawValue
                return .failed(failure)
            }
            log.conflicts += 1
            guard let remote = backend.read(directory) else {
                log.outcome = "conflict_unreadable"
                return .conflictUnreadable
            }
            broken += remote.broken.filter { !broken.contains($0) }
            next = MemoryMerge.merge(base: base, local: next, remote: remote)
            base = remote
        }
        log.outcome = "conflict_gave_up"
        return .gaveUp(latest: backend.read(directory))
    }

    /// 探一下 memory/.lock 现在是不是被别的进程占着（只给日志用）：非阻塞地拿一下马上放掉。
    private static func lockBusy(_ directory: URL) -> Bool {
        let path = directory.appendingPathComponent("memory/.lock").path
        let fd = open(path, O_RDONLY)
        guard fd >= 0 else { return false }
        defer { close(fd) }
        guard flock(fd, LOCK_EX | LOCK_NB) == 0 else { return errno == EWOULDBLOCK }
        flock(fd, LOCK_UN)
        return false
    }
}
