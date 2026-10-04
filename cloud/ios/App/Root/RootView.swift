// 主 App 根视图：底部三栏「记得 · 通讯录 · 我」（RootTab）。「记得」暂放键盘记住的事，「通讯录」是占位。
// 启动时读一次记忆，从后台回到前台再读（App 在后台期间键盘可能记过一笔）；刚启动的 inactive → active 不算，免得连读两遍。
// 写记忆失败的提示框挂在 TabView 上，弹出的编辑页各自另挂一个。

import SwiftUI

struct RootView: View {
    @Environment(\.scenePhase) private var scenePhase

    @State private var memory = MemoryStore()

    @State private var wasInBackground = false

    var body: some View {
        TabView {
            ForEach(RootTab.allCases, id: \.self) { tab in
                // 链接与按钮色照设计稿 a{color:var(--accent-ink)}；挂在各页上而不是 TabView 上，免得盖掉 TabBarStyle 的选中色
                page(tab)
                    .tint(ColorUsage.appLink.role.color)
                    .tabItem { Label(tab.title, image: tab.icon) }
            }
        }
        .memoryAlert(memory)
        .task { await memory.reload() }
        .onChange(of: scenePhase) { _, phase in
            if phase == .background { wasInBackground = true }
            guard phase == .active, wasInBackground else { return }
            wasInBackground = false
            Task { await memory.reload() }
        }
    }

    @ViewBuilder
    private func page(_ tab: RootTab) -> some View {
        switch tab {
        case .remember: MemoryHomeView(store: memory)
        case .contacts: ContactsPlaceholderView()
        case .me: MeView()
        }
    }
}
