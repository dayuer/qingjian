// 操作失败后界面要做的事（message 总是显示）；由 `AccountStore.reaction(for:step:)` 按 code 决定。

import Foundation

enum AccountReaction: Equatable {
    /// 只显示 message，登录状态不变（开关操作失败时由调用方回滚开关）。
    case showMessage

    /// 验证码输错：清空输入框、焦点回到输入框，留在验证码页。
    case retryCode

    /// 当天锁定：回到未发码状态，邮箱区域禁用，引导用 Apple 登录。
    case lockEmail

    /// 令牌失效：桥已清令牌，重取一次状态回到未登录。
    case signOutLocally
}
