// 「我」Tab（启用键盘交给首次引导，App 判断不了键盘加没加）：记忆要开完全访问的说明、键盘设置与关于（账号入口先藏起来，试打框在键盘设置里），最底下不显眼的「重新看引导」。

import SwiftUI
import UIKit

struct MeView: View {
    /// 「账号」入口先藏起来：界面里不出现账号（UI 清单约束 5），AccountView 的代码留着，T9 换成开通云服务的流程。
    static let showsAccountEntry = false

    static let replayOnboardingTitle = "重新看引导"

    /// 「重新看引导」：由 RootView 盖上首次引导。设计稿 05 的 2j 没有这一项，有意加的（UI 清单约束 7）。
    var replayOnboarding: () -> Void = {}

    @State private var store = SettingsStore()

    @State private var account = AccountStore()

    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                // 标题放在 Form **外面**：放进 Section 会被 List 的分节内边距往下推 35pt，跟另外两页对不齐
                PageHeader(title: "我")
                Form {
                    // 记忆要完全访问（键盘读 App Group 里的卡片）；和键盘面板同一段话，这里能直接跳到素笺的设置页
                    Section {
                        Text(ScopeDisplay.fullAccessExplanation)
                        Text(ScopeDisplay.fullAccessPath)
                            .font(AppFont.footnote)
                            .foregroundStyle(.secondary)
                        Button("去开启") {
                            if let url = URL(string: UIApplication.openSettingsURLString) {
                                UIApplication.shared.open(url)
                            }
                        }
                    } header: {
                        Text("记忆")
                    }
                    Section {
                        if store.available {
                            NavigationLink("键盘设置") { KeyboardSettingsView(store: store) }
                            if Self.showsAccountEntry {
                                NavigationLink("账号") { AccountView(store: account) }
                            }
                        } else {
                            Text("这个安装包没有开通 App Group，设置改不到键盘上。").foregroundStyle(.secondary)
                        }
                        NavigationLink("关于") { AboutView() }
                    } header: {
                        Text("设置")
                    }
                    Section {
                        Button(Self.replayOnboardingTitle, action: replayOnboarding)
                            .font(AppFont.footnote)
                            .foregroundStyle(.secondary)
                            .frame(maxWidth: .infinity)
                            .listRowBackground(Color.clear)
                    }
                }
            }
            .toolbar(.hidden, for: .navigationBar)
        }
    }
}
