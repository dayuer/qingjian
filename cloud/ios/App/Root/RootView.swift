// 主 App 根视图：底部三栏「记得 · 通讯录 · 我」（RootTab）。「记得」暂放键盘记住的事，「通讯录」是占位。
// 启动时读一次记忆，从后台回到前台再读（App 在后台期间键盘可能记过一笔）；刚启动的 inactive → active 不算，免得连读两遍。
// 写记忆失败的提示框挂在 TabView 上，弹出的编辑页各自另挂一个。
// 首次引导：第一次读完记忆后按 OnboardingGate 决定要不要全屏盖上；关掉（走完或跳过）就记成看过，落到「记得」。「我」页可以重新看。

import SwiftUI

struct RootView: View {
    @Environment(\.scenePhase) private var scenePhase

    @State private var memory = MemoryStore()

    @State private var wasInBackground = false

    @State private var tab = RootTab.remember

    @State private var showsOnboarding = false

    /// 只在启动后第一次读完时判断一次：全屏盖收起时 .task 可能再跑一遍，不能再判断（那时 onDismiss 可能还没记成看过）。
    @State private var onboardingChecked = false

    @AppStorage(OnboardingGate.doneKey) private var onboardingDone = false

    var body: some View {
        TabView(selection: $tab) {
            ForEach(RootTab.allCases, id: \.self) { tab in
                // 链接与按钮色照设计稿 a{color:var(--accent-ink)}；挂在各页上而不是 TabView 上，免得盖掉 TabBarStyle 的选中色
                page(tab)
                    .tint(ColorUsage.appLink.role.color)
                    .tabItem { Label(tab.title, image: tab.icon) }
                    .tag(tab)
            }
        }
        .memoryAlert(memory)
        .task {
            await memory.reload()
            guard !onboardingChecked else { return }
            onboardingChecked = true
            // 读失败时 contacts 给 nil，OnboardingGate 当作没有对象，照样出引导
            let contacts = memory.canEdit ? memory.snapshot.contacts : nil
            showsOnboarding = OnboardingGate.shouldShow(done: onboardingDone, contacts: contacts)
        }
        .fullScreenCover(isPresented: $showsOnboarding, onDismiss: finishOnboarding) {
            OnboardingView(store: memory)
        }
        .onChange(of: scenePhase) { _, phase in
            if phase == .background { wasInBackground = true }
            guard phase == .active, wasInBackground else { return }
            wasInBackground = false
            Task { await memory.reload() }
        }
        // 没显式指定字体的 Text 也用 MiSans；挂在最外层，引导的全屏盖与各页弹层一并继承
        .environment(\.font, AppFont.body)
    }

    @ViewBuilder
    private func page(_ tab: RootTab) -> some View {
        switch tab {
        case .remember: MemoryHomeView(store: memory)
        case .contacts: ContactsPlaceholderView()
        case .me: MeView(replayOnboarding: { showsOnboarding = true })
        }
    }

    private func finishOnboarding() {
        onboardingDone = true
        tab = .remember
    }
}
