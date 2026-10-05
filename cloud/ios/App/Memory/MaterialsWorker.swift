// 素材的读删放在后台串行做：桥等 memory/.lock 最多 2 秒，不能卡界面。做法同 MemoryWorker：自己的串行队列当执行器，桥的同步调用不占协作线程池。
// 日志只记成败与错误码，不记原话。

import Dispatch
import Foundation
import os

actor MaterialsWorker {
    private static let log = Logger(subsystem: "sujian.synon.ai", category: "materials")

    private let queue = DispatchSerialQueue(label: "sujian.synon.ai.materials")

    private let backend: MaterialsBackend

    nonisolated var unownedExecutor: UnownedSerialExecutor { queue.asUnownedSerialExecutor() }

    init(backend: MaterialsBackend) {
        self.backend = backend
    }

    func list(_ directory: URL, contactId: String) -> MaterialList? {
        let list = backend.list(directory, contactId)
        if list == nil { Self.log.error("待整理读不出来") }
        return list
    }

    func delete(_ directory: URL, contactId: String, clientId: String) -> MemoryFailure? {
        let failure = backend.delete(directory, contactId, clientId)
        if let failure { Self.log.error("待整理删失败 \(failure.code.rawValue, privacy: .public)") }
        return failure
    }
}
