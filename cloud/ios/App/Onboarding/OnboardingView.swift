// 首次引导：介绍 → 开启键盘 → 免费版与云服务 → 第一个对象（OnboardingStep），RootView 用 fullScreenCover 盖在上面。
// 第四步直接放 ContactEditor（建在恋爱场景）：它的「取消」与存好后的 dismiss 关掉的就是这层全屏盖，所以跳过与建好都由 RootView 的 onDismiss 收尾。
// 包一层 NavigationStack 只为「了解云服务」能推到 CloudIntroView，各步自己不显示导航栏。

import SwiftUI

struct OnboardingView: View {
    let store: MemoryStore

    @State private var step = OnboardingStep.intro

    @State private var showsCloudIntro = false

    var body: some View {
        NavigationStack {
            page
                .id(step)
                .transition(.asymmetric(insertion: .move(edge: .trailing), removal: .move(edge: .leading)))
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
                .background(Color(.systemBackground))
                .toolbar(.hidden, for: .navigationBar)
                .navigationDestination(isPresented: $showsCloudIntro) { CloudIntroView() }
        }
    }

    @ViewBuilder
    private var page: some View {
        switch step {
        case .intro: IntroStep(onStart: advance)
        case .keyboard: KeyboardStep(onNext: advance)
        case .plan: PlanStep(onNext: advance, onLearnCloud: { showsCloudIntro = true })
        case .contact:
            VStack(spacing: 0) {
                OnboardingProgressBar(step: .contact)
                    .padding(.horizontal, 24)
                    .padding(.top, 16)
                ContactEditor(store: store)
            }
        }
    }

    private func advance() {
        guard let next = step.next else { return }
        withAnimation(.easeInOut(duration: 0.3)) { step = next }
    }
}
