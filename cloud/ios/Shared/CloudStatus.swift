// 开没开素笺云：cloud.toml 里有没有服务器地址与登录令牌（桥的 qj_cloud_configured，与键盘建云端客户端时同一个判断，不联网）。
// App「待整理」下面要不要放开通引导、键盘记一笔的 toast 写「明早整理」还是「开通素笺云后整理」都按它。

import Foundation
import QingjianBridge

enum CloudStatus {
    static func configured(file: URL? = SharedStore.cloudFile) -> Bool {
        guard let file else { return false }
        return file.path.withCString { qj_cloud_configured($0) }
    }
}
