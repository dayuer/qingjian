// 未登录时邮箱登录走到哪一步：填邮箱，或已发码等输入验证码。

import Foundation

enum LoginStep: Equatable {
    case email

    case code
}
