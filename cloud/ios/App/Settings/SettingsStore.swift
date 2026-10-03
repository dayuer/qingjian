// 设置页的状态：从 App Group 里的 config.toml / cloud.toml 读出，改一项写一次。
// 键盘每次轮询按修改时间重读 config.toml；同一个文件经青简 Cloud 与 Mac 同步，所以打开设置页时重读一遍。

import Foundation
import Observation

@MainActor
@Observable
final class SettingsStore {
    private(set) var settings: KeyboardSettings?

    var cloud: CloudSettings?

    /// 设置页要弹出来的写入失败原因。
    var error: String?

    /// 签名没带 App Group：主 App 与键盘不共享文件，设置页改不了键盘。
    var available: Bool { SharedStore.directory != nil }

    init() {
        Self.seedCloudConfig()
        reload()
    }

    func reload() {
        guard let config = SharedStore.configFile, let cloudFile = SharedStore.cloudFile else { return }
        SharedStore.ensure(SharedStore.directory)
        settings = SettingsBridge.readSettings(config: config, dicts: Self.dictsDirectory)
        cloud = SettingsBridge.readCloud(cloudFile)
    }

    /// 改一项就写回；写不进去（如自定义短语不合法）时撤回这次改动，返回原因。
    @discardableResult
    func update(_ change: (inout KeyboardSettings) -> Void) -> String? {
        guard var next = settings, let config = SharedStore.configFile else { return "设置文件不可用" }
        change(&next)
        if let failure = SettingsBridge.writeSettings(next, config: config) { return failure }
        settings = next
        return nil
    }

    func saveCloud() -> String? {
        guard let cloud, let file = SharedStore.cloudFile else { return "配置文件不可用" }
        return SettingsBridge.writeCloud(cloud, file: file)
    }

    /// 随包领域词库在键盘扩展的 Data/dicts 里，主 App 直接读扩展包。
    private static var dictsDirectory: URL {
        Bundle.main.builtInPlugInsURL!
            .appendingPathComponent("Keyboard.appex/Data/dicts", isDirectory: true)
    }

    /// 构建时打进键盘的 cloud.toml（开发者本机的 cloud.local.toml）只作种子：共享目录里还没有时拷一份。
    private static func seedCloudConfig() {
        guard let target = SharedStore.cloudFile,
              !FileManager.default.fileExists(atPath: target.path),
              let bundled = Bundle.main.builtInPlugInsURL?
                .appendingPathComponent("Keyboard.appex/Data/cloud.toml"),
              FileManager.default.fileExists(atPath: bundled.path)
        else { return }
        SharedStore.ensure(SharedStore.directory)
        try? FileManager.default.copyItem(at: bundled, to: target)
    }
}
