// 改写进行到哪了：没在组字时工具栏显示当前技能名，点了等结果，结果出来点一下替换原文。
// 没成功分两种说法：网络失败（`.failed`）与被那道闸丢掉的（`.rejected`，模型给的不合用）。
// `skill` 是发出这次改写时用的技能名，写在改写条上（换一个技能重改时看得见用的是哪个）。

enum RewriteState: Equatable {
    case idle

    /// 已发出，`original` 是要被替换的那段（光标前的文字），`skill` 是这次用的技能名。
    case pending(original: String, skill: String)

    case ready(original: String, skill: String, result: String)

    /// 网络失败、服务器没开大模型。
    case failed

    /// 模型给的不合用（空的、或比原文长出一大截），已丢掉；说法与网络失败不同。
    case rejected
}
