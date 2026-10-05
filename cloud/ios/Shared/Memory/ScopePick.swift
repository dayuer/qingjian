// 切人时怎么定当前对象，与 qj_scope_set 的 contact_id 参数对应：保持现在选的人不变（NULL）、明确不指定（空字符串）、指定一个人。

enum ScopePick: Equatable, Sendable {
    /// 保持现在选的人不变（幂等）。原来是「回到这个场景上次选的人」，场景去掉后没有「上次」了。
    case keep

    case nobody

    case contact(String)

    /// 传给桥的参数；nil 表示传空指针。
    var argument: String? {
        switch self {
        case .keep: nil
        case .nobody: ""
        case .contact(let id): id
        }
    }
}
