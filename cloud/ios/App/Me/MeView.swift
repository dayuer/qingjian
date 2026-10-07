// 「我」Tab（设计稿 05 的 2j）：记忆要开完全访问的说明与「去开启」、素笺云服务的入口、
// 键盘设置与关于（试打框在键盘设置里），最底下不显眼的「重新看引导」。
// 界面里不再出现「账号」：开通云服务那条路是建空间 + 匹配码（见 `App/Space/`），没有登录；
// 撤回同意、删云端数据都在「素笺云服务」页里（05 的 3c）。

import SwiftUI
import UIKit

struct MeView: View {
    static let replayOnboardingTitle = "重新看引导"

    /// 「素笺云服务」那一行的标题与两种状态（测试与截图走查用）。
    static let cloudTitle = SpaceWording.entryTitle

    static func cloudStatus(signedIn: Bool?) -> String {
        signedIn == true ? SpaceWording.entryOpened : SpaceWording.entryClosed
    }

    /// 「重新看引导」：由 RootView 盖上首次引导。设计稿 05 的 2j 没有这一项，有意加的（UI 清单约束 7）。
    var replayOnboarding: () -> Void = {}

    @State private var store = SettingsStore()

    @State private var space = SpaceStore()

    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                PageHeader(title: "我")
                PaperPage(spacing: 10, top: 8) {
                    PaperSectionTitle(text: "记忆")
                    PaperGroup { memoryRow }
                    PaperGroup {
                        NavigationLink {
                            SpaceEntryView(store: space)
                        } label: {
                            PaperRow(
                                title: Self.cloudTitle, side: Self.cloudStatus(signedIn: space.signedIn),
                                chevron: true)
                        }
                        .buttonStyle(.plain)
                    }
                    PaperSectionTitle(text: "设置")
                    PaperGroup {
                        if store.available {
                            NavigationLink {
                                KeyboardSettingsView(store: store)
                            } label: {
                                PaperRow(title: "键盘设置", chevron: true)
                            }
                            .buttonStyle(.plain)
                        } else {
                            PaperRow(title: "这个安装包没有开通 App Group，设置改不到键盘上。", titleColor: Theme.ink3)
                        }
                        PaperRowLine()
                        NavigationLink {
                            AboutView()
                        } label: {
                            PaperRow(title: "关于", chevron: true)
                        }
                        .buttonStyle(.plain)
                    }
                    Button(Self.replayOnboardingTitle, action: replayOnboarding)
                        .font(AppFont.font(size: 12.5))
                        .foregroundStyle(Theme.ink3)
                        .frame(maxWidth: .infinity)
                        .padding(.top, 10)
                }
            }
            .task { space.refresh() }
            .background(Theme.paper)
            .toolbar(.hidden, for: .navigationBar)
        }
    }

    /// 记忆那一行：标题、一句说明、开到完全访问的路径，右边线框小按钮「去开启」。
    private var memoryRow: some View {
        HStack(alignment: .top, spacing: 12) {
            VStack(alignment: .leading, spacing: 2) {
                Text(ScopeDisplay.fullAccessTitle)
                    .font(AppFont.font(size: 15))
                    .foregroundStyle(Theme.ink)
                Text(ScopeDisplay.fullAccessNote)
                    .font(AppFont.font(size: 12.5))
                    .foregroundStyle(Theme.ink3)
                Text(ScopeDisplay.fullAccessPath)
                    .font(AppFont.font(size: 12))
                    .foregroundStyle(Theme.ink3)
                    .padding(.top, 2)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            PaperLineButton(title: "去开启", height: 30) {
                if let url = URL(string: UIApplication.openSettingsURLString) {
                    UIApplication.shared.open(url)
                }
            }
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 12)
    }
}
