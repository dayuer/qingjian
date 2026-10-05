// 调 Rust 桥的空间接口（qj_space_* / qj_pair_*）：都是阻塞的网络请求，只在后台任务里调（见 SpaceStore）。
// 令牌留在桥与 cloud.toml 之间，不经过 Swift。

import Foundation
import QingjianBridge

enum PairBridge {
    /// 建空间；成功这台设备就已登录（令牌写进 cloud.toml）。成功返回 nil，失败返回原因。
    static func createSpace(_ file: URL, device: String, crossBorderConsented: Bool) -> BridgeFailure? {
        failure(file.path.withCString { f in
            device.withCString { d in qj_space_create(f, d, crossBorderConsented) }
        })
    }

    static func pairCode(_ file: URL) -> Result<PairReply.Code, BridgeFailure> {
        PairReply.result(SettingsBridge.take(file.path.withCString { qj_pair_code($0) }))
    }

    static func pairJoin(_ file: URL, code: String, device: String) -> Result<PairReply.Ticket, BridgeFailure> {
        PairReply.result(SettingsBridge.take(file.path.withCString { f in
            code.withCString { c in device.withCString { d in qj_pair_join(f, c, d) } }
        }))
    }

    static func pairPoll(_ file: URL, requestId: String, secret: String) -> Result<PairReply.Poll, BridgeFailure> {
        PairReply.result(SettingsBridge.take(file.path.withCString { f in
            requestId.withCString { r in secret.withCString { s in qj_pair_poll(f, r, s) } }
        }))
    }

    static func pairRequests(_ file: URL) -> Result<[PairReply.Request], BridgeFailure> {
        PairReply.result(SettingsBridge.take(file.path.withCString { qj_pair_requests($0) }))
    }

    static func pairDecide(_ file: URL, requestId: String, allow: Bool) -> BridgeFailure? {
        failure(file.path.withCString { f in
            requestId.withCString { r in qj_pair_decide(f, r, allow) }
        })
    }

    /// 本机有没有拿到过会话（只看 cloud.toml，不联网）。
    static func signedIn(_ file: URL) -> Bool {
        file.path.withCString { qj_cloud_signed_in($0) }
    }

    /// NULL 是成功。
    private static func failure(_ raw: UnsafeMutablePointer<CChar>?) -> BridgeFailure? {
        BridgeFailure.decode(SettingsBridge.take(raw))
    }
}
