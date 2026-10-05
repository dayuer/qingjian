// 改写「没成功」时改写条上那句话。两种原因用户能做的事不一样：网络问题要检查网络，模型给的不合用要换个技能再试。

enum RewriteWording {
    /// 网络失败、服务器没开大模型。
    static let failed = "改写没成功，检查网络后再试"

    /// 模型给的不合用（空的、或比原文长出一大截），已经丢掉了。
    static let rejected = "没改好，换一个试试"

    /// 这个状态在改写条上要说的那句；没有话说的（空闲、等待、结果）给 `nil`。
    static func note(for state: RewriteState) -> String? {
        switch state {
        case .failed: failed
        case .rejected: rejected
        case .idle, .pending, .ready: nil
        }
    }
}
