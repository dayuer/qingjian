// 切场景时怎么定对象，与 qj_scope_set 的 contact_id 参数对应：回到这个场景上次选的人（NULL）、明确不指定（空字符串）、指定一个人。

enum ScopePick: Equatable, Sendable {
    case last

    case nobody

    case contact(String)

    /// 传给桥的参数；nil 表示传空指针。
    var argument: String? {
        switch self {
        case .last: nil
        case .nobody: ""
        case .contact(let id): id
        }
    }
}
