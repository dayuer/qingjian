// 设置页的状态：从 App Group 里的 config.toml 读出，改一项写一次。
// 键盘每次轮询按修改时间重读 config.toml；同一个文件经素笺云 与 Mac 同步，所以打开设置页时重读一遍。

import Foundation
import Observation

@MainActor
@Observable
final class SettingsStore {
    private(set) var settings: KeyboardSettings?

    /// 设置页要弹出来的写入失败原因。
    var error: String?

    /// 签名没带 App Group：主 App 与键盘不共享文件，设置页改不了键盘。
    var available: Bool { SharedStore.directory != nil }

    init() {
        Self.seedCloudConfig()
        reload()
    }

    func reload() {
        guard let config = SharedStore.configFile else { return }
        SharedStore.ensure(SharedStore.directory)
        settings = SettingsBridge.readSettings(config: config, dicts: Self.dictsDirectory)
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

    /// 随包领域词库在键盘扩展的 Data/dicts 里，主 App 直接读扩展包。
    private static var dictsDirectory: URL {
        Bundle.main.builtInPlugInsURL!
            .appendingPathComponent("Keyboard.appex/Data/dicts", isDirectory: true)
    }

    /// 随包的 cloud.toml 只写了服务器地址（构建时定），共享目录里还没有时拷一份；令牌由账号页登录写入。
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
