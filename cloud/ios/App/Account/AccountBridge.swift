// 调 Rust 桥的账号接口（qj_account_*）：都是阻塞的网络请求，只在后台任务里调（见 AccountStore）。
// 操作成功返回 nil，失败返回 BridgeFailure（code 加给用户看的文案）；令牌留在桥与 cloud.toml 之间，不经过 Swift。

import Foundation
import QingjianBridge

enum AccountBridge {
    /// 清空云端输入记录，同时删本机日志与上传进度。`userDir` 是学习数据目录（cloud.toml 的上级）。
    static func clearInputLog(cloudFile: URL, userDir: URL) -> BridgeFailure? {
        take(cloudFile.path.withCString { c in
            userDir.path.withCString { u in qj_input_log_clear(u, c) }
        })
    }

    static func status(_ file: URL) -> AccountState? {
        SettingsBridge.decode(SettingsBridge.take(file.path.withCString { qj_account_status($0) }))
    }

    static func signInApple(
        _ file: URL, identityToken: String, authorizationCode: String, nonce: String, device: String,
        crossBorderConsented: Bool
    ) -> BridgeFailure? {
        take(file.path.withCString { f in
            identityToken.withCString { t in
                authorizationCode.withCString { c in
                    nonce.withCString { n in
                        device.withCString { qj_account_sign_in_apple(f, t, c, n, $0, crossBorderConsented) }
                    }
                }
            }
        })
    }

    static func emailStart(_ file: URL, email: String, crossBorderConsented: Bool) -> BridgeFailure? {
        take(file.path.withCString { f in
            email.withCString { qj_account_email_start(f, $0, crossBorderConsented) }
        })
    }

    static func emailVerify(
        _ file: URL, email: String, code: String, device: String, crossBorderConsented: Bool
    ) -> BridgeFailure? {
        take(file.path.withCString { f in
            email.withCString { e in
                code.withCString { c in
                    device.withCString { qj_account_email_verify(f, e, c, $0, crossBorderConsented) }
                }
            }
        })
    }

    static func setConsent(_ file: URL, feature: String, enabled: Bool) -> BridgeFailure? {
        take(file.path.withCString { f in
            feature.withCString { qj_account_set_consent(f, $0, enabled) }
        })
    }

    static func revokeSession(_ file: URL, id: Int64) -> BridgeFailure? {
        take(file.path.withCString { qj_account_revoke_session($0, id) })
    }

    static func signOut(_ file: URL) -> BridgeFailure? {
        take(file.path.withCString { qj_account_sign_out($0) })
    }

    static func deleteAccount(_ file: URL) -> BridgeFailure? {
        take(file.path.withCString { qj_account_delete($0) })
    }

    /// 取走桥返回的字符串并解成失败；NULL 是成功。
    private static func take(_ raw: UnsafeMutablePointer<CChar>?) -> BridgeFailure? {
        BridgeFailure.decode(SettingsBridge.take(raw))
    }
}
