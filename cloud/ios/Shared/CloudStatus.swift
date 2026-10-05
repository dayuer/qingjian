// 素材会不会被整理：cloud.toml 里有服务器地址与登录令牌，并且同意了「记忆」（桥的 qj_memory_cloud_ready，不联网）。
// App「待整理」下面要不要放开通引导、键盘记一笔的 toast 写「明早整理」还是「开通素笺云后整理」都按它：没同意时素材不上传，不能说明早整理。

import Foundation
import QingjianBridge

enum CloudStatus {
    static func memoryReady(file: URL? = SharedStore.cloudFile) -> Bool {
        guard let file else { return false }
        return file.path.withCString { qj_memory_cloud_ready($0) }
    }
}
