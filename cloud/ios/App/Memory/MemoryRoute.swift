// 「记住的」页里的导航：某个人的详情、某个人的设置。用路径而不是嵌套的 NavigationLink，人被忘掉时首页能一次退掉这个人的所有页面。

enum MemoryRoute: Hashable {
    case contact(String)

    case settings(String)

    var contactId: String {
        switch self {
        case .contact(let id), .settings(let id): id
        }
    }
}
