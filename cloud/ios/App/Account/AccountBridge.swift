// 调 Rust 桥的账号接口（qj_account_*）：都是阻塞的网络请求，只在后台任务里调（见 AccountStore）。
// 操作成功返回 nil，失败返回给用户看的原因；令牌留在桥与 cloud.toml 之间，不经过 Swift。

import Foundation
import QingjianBridge

enum AccountBridge {
    static func status(_ file: URL) -> AccountState? {
        SettingsBridge.decode(SettingsBridge.take(file.path.withCString { qj_account_status($0) }))
    }

    static func signInApple(
        _ file: URL, identityToken: String, authorizationCode: String, nonce: String, device: String
    ) -> String? {
        SettingsBridge.take(file.path.withCString { f in
            identityToken.withCString { t in
                authorizationCode.withCString { c in
                    nonce.withCString { n in
                        device.withCString { qj_account_sign_in_apple(f, t, c, n, $0) }
                    }
                }
            }
        })
    }

    static func emailStart(_ file: URL, email: String) -> String? {
        SettingsBridge.take(file.path.withCString { f in
            email.withCString { qj_account_email_start(f, $0) }
        })
    }

    static func emailVerify(_ file: URL, email: String, code: String, device: String) -> String? {
        SettingsBridge.take(file.path.withCString { f in
            email.withCString { e in
                code.withCString { c in
                    device.withCString { qj_account_email_verify(f, e, c, $0) }
                }
            }
        })
    }

    static func setConsent(_ file: URL, feature: String, enabled: Bool) -> String? {
        SettingsBridge.take(file.path.withCString { f in
            feature.withCString { qj_account_set_consent(f, $0, enabled) }
        })
    }

    static func revokeSession(_ file: URL, id: Int64) -> String? {
        SettingsBridge.take(file.path.withCString { qj_account_revoke_session($0, id) })
    }

    static func signOut(_ file: URL) -> String? {
        SettingsBridge.take(file.path.withCString { qj_account_sign_out($0) })
    }

    static func deleteAccount(_ file: URL) -> String? {
        SettingsBridge.take(file.path.withCString { qj_account_delete($0) })
    }
}
