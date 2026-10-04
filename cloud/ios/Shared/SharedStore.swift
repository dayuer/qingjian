// 主 App 与键盘共用的 App Group 目录：config.toml（设置页写、键盘读、经素笺云 与 Mac 同步）、
// cloud.toml（连接配置），开了完全访问时学习数据也放这里（没开时键盘写不了共享目录，学习数据留在扩展自己的容器）。

import Foundation

enum SharedStore {
    static let groupIdentifier = "group.app.qingjian.cloud"

    /// App Group 容器下的 `Library/Application Support/Qingjian`；签名没带 App Group 时为 nil。
    static var directory: URL? {
        FileManager.default.containerURL(forSecurityApplicationGroupIdentifier: groupIdentifier)?
            .appendingPathComponent("Library/Application Support/Qingjian", isDirectory: true)
    }

    static var configFile: URL? { directory?.appendingPathComponent("config.toml") }

    static var cloudFile: URL? { directory?.appendingPathComponent("cloud.toml") }

    /// 本进程自己容器里的 `Library/Application Support/Qingjian`（键盘没完全访问时的学习数据目录）。
    static var localDirectory: URL? {
        FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first?
            .appendingPathComponent("Qingjian", isDirectory: true)
    }

    static func ensure(_ directory: URL?) {
        guard let directory else { return }
        try? FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
    }
}
