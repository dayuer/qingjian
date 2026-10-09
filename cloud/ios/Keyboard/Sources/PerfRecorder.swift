// 真机验收用的性能记录：每 250ms 一拍记键盘扩展自己的 phys_footprint，每键耗时、停键到重排各记一行，
// 追加到 App Group 容器的 perf.tsv（`时刻毫秒\t种类\t数值`），用 devicectl 从手机拷出来算峰值与 p95。
// Instruments 认不了比本机 Xcode 新的系统时用它；要开「完全访问」才写得进共享容器。只在 `Diagnostics.enabled` 时记。

import Darwin
import Foundation
import os

@MainActor
final class PerfRecorder {
    static let shared = PerfRecorder()

    /// 同样的数也写进系统日志（`sujian.keyboard` / `perf`）：共享容器要开「完全访问」才写得进，日志不用。
    private static let log = Logger(subsystem: "sujian.keyboard", category: "perf")

    private let url: URL?
    private var pending: [String] = []
    private let started = DispatchTime.now()

    private init() {
        url = FileManager.default
            .containerURL(forSecurityApplicationGroupIdentifier: SharedStore.groupIdentifier)?
            .appendingPathComponent("perf.tsv")
    }

    private func add(_ kind: String, _ value: Double) {
        guard Diagnostics.enabled else { return }
        let ms = Double(DispatchTime.now().uptimeNanoseconds - started.uptimeNanoseconds) / 1e6
        pending.append(String(format: "%.0f\t%@\t%.2f", ms, kind, value))
    }

    func key(_ ms: Double) { add("key_ms", ms) }

    func rescore(_ ms: Double) { add("rescore_ms", ms) }

    func mark(_ stage: String) {
        guard Diagnostics.enabled else { return }
        let mb = Self.footprintMB()
        Self.log.info("mark=\(stage, privacy: .public) footprint_mb=\(mb, format: .fixed(precision: 2), privacy: .public)")
        add("mark_" + stage, mb)
    }

    /// 记一拍内存并把攒下的行写盘（控制器的 250ms 定时器调）。
    func tick() {
        guard Diagnostics.enabled else { return }
        let mb = Self.footprintMB()
        Self.log.info("footprint_mb=\(mb, format: .fixed(precision: 2), privacy: .public)")
        add("footprint_mb", mb)
        guard let url, !pending.isEmpty else { return }
        let text = pending.joined(separator: "\n") + "\n"
        pending.removeAll()
        if let handle = try? FileHandle(forWritingTo: url) {
            handle.seekToEndOfFile()
            handle.write(Data(text.utf8))
            try? handle.close()
        } else {
            try? Data(text.utf8).write(to: url)
        }
    }

    /// 本进程的 phys_footprint（MB）：jetsam 按它杀扩展。
    static func footprintMB() -> Double {
        var info = task_vm_info_data_t()
        var count = mach_msg_type_number_t(MemoryLayout<task_vm_info_data_t>.size / MemoryLayout<natural_t>.size)
        let result = withUnsafeMutablePointer(to: &info) {
            $0.withMemoryRebound(to: integer_t.self, capacity: Int(count)) {
                task_info(mach_task_self_, task_flavor_t(TASK_VM_INFO), $0, &count)
            }
        }
        return result == KERN_SUCCESS ? Double(info.phys_footprint) / 1_048_576 : -1
    }
}
