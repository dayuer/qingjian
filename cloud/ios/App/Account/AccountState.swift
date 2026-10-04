// 账号页的数据，与桥的 `AccountStatus` JSON 对应（蛇形命名由 SettingsBridge.decode 转换）。令牌不在里面。

import Foundation

struct AccountState: Codable, Equatable {
    var server: String

    var signedIn: Bool

    /// 服务器上的开关；没登录或这次取不到时是 cloud.toml 里的。
    var consents: Consents

    var identities: [AccountIdentity]

    var sessions: [AccountDevice]

    /// 这次没能从服务器取到账号的原因。
    var error: String?

    /// `error` 的种类；没有错误时没有。
    var errorCode: AccountFailure.Code?
}
