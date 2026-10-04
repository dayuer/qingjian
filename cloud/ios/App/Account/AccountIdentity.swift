// 账号的一种登录方式。

import Foundation

struct AccountIdentity: Codable, Equatable, Hashable {
    /// apple / email。
    var provider: String

    /// 邮箱地址；Apple 没给邮箱时为空。
    var label: String?

    var title: String {
        switch provider {
        case "apple": "Apple"
        case "email": "邮箱"
        default: provider
        }
    }
}
