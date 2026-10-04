// 底部三栏「记得 · 通讯录 · 我」的标题、图标与顺序（02 第 2 轮 2a、2b，05 的 2j）。
// 图标是 Assets.xcassets 里的 template 矢量图，从设计稿 theme.css 的 .i-cal / .i-book / .i-me 导出。

enum RootTab: CaseIterable, Hashable {
    case remember

    case contacts

    case me

    var title: String {
        switch self {
        case .remember: "记得"
        case .contacts: "通讯录"
        case .me: "我"
        }
    }

    /// Assets.xcassets 里的图片名。
    var icon: String {
        switch self {
        case .remember: "tab-calendar"
        case .contacts: "tab-contacts"
        case .me: "tab-me"
        }
    }
}
