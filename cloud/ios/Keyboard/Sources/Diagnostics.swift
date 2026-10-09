// 真机诊断（性能日志、perf.tsv、存活计数）的开关：Debug 构建一直开；Release 默认关，
// App Group 容器根下有 `perf-on` 文件时才开（用 devicectl 拷进去，键盘要开「完全访问」才看得见共享容器）。

import Foundation

enum Diagnostics {
    /// 进程起来时定一次：键盘扩展常被系统杀掉重起，放进或删掉 `perf-on` 后下次调出键盘生效。
    static let enabled: Bool = {
        #if DEBUG
            return true
        #else
            guard
                let root = FileManager.default.containerURL(
                    forSecurityApplicationGroupIdentifier: SharedStore.groupIdentifier)
            else { return false }
            return FileManager.default.fileExists(atPath: root.appendingPathComponent("perf-on").path)
        #endif
    }()
}
