// 键盘该用哪几个文件：学习数据目录、config.toml、cloud.toml。
// 开了完全访问才能写 App Group，学习数据第一次进共享目录时从扩展容器搬过去（只在共享目录还没有时搬，不覆盖）。

import Foundation

struct UserData {
    let userDirectory: URL?

    let configFile: URL?

    let cloudFile: URL?

    /// 同步要联网，本来就要完全访问；没开时学习数据留在扩展容器，只读共享目录里的设置。
    static func resolve(fullAccess: Bool, bundledData: URL) -> UserData {
        let local = SharedStore.localDirectory
        let shared = SharedStore.directory
        let user: URL?
        if fullAccess, let shared {
            SharedStore.ensure(shared)
            migrate(from: local, to: shared)
            user = shared
        } else {
            SharedStore.ensure(local)
            user = local
        }
        // 主 App 打开过一次就会把随包的 cloud.toml 种到共享目录；还没打开过时直接用随包的
        let bundledCloud = bundledData.appendingPathComponent("cloud.toml")
        let cloud = [SharedStore.cloudFile, bundledCloud].compactMap { $0 }
            .first { FileManager.default.fileExists(atPath: $0.path) }
        return UserData(
            userDirectory: user, configFile: SharedStore.configFile ?? user?.appendingPathComponent("config.toml"),
            cloudFile: cloud)
    }

    /// 旧版本把学习数据与同步基线放在扩展容器：共享目录里还没有 user.tsv 时整份拷过去。
    private static func migrate(from local: URL?, to shared: URL) {
        let manager = FileManager.default
        guard let local,
              manager.fileExists(atPath: local.appendingPathComponent("user.tsv").path),
              !manager.fileExists(atPath: shared.appendingPathComponent("user.tsv").path),
              let items = try? manager.contentsOfDirectory(at: local, includingPropertiesForKeys: nil)
        else { return }
        for item in items {
            let target = shared.appendingPathComponent(item.lastPathComponent)
            // config.toml 以共享目录（设置页）为准
            guard !manager.fileExists(atPath: target.path) else { continue }
            try? manager.copyItem(at: item, to: target)
        }
    }
}
