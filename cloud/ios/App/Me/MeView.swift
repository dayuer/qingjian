// 「我」Tab：启用键盘的步骤、记忆要开完全访问的说明、键盘设置、关于、试打框（账号入口先藏起来）。

import SwiftUI
import UIKit

struct MeView: View {
    /// 「账号」入口先藏起来：界面里不出现账号（UI 清单约束 5），AccountView 的代码留着，T9 换成开通云服务的流程。
    static let showsAccountEntry = false

    @State private var draft = ""

    @State private var store = SettingsStore()

    @State private var account = AccountStore()

    var body: some View {
        NavigationStack {
            Form {
                Section("启用键盘") {
                    Label("打开「设置 → 通用 → 键盘 → 键盘」", systemImage: "1.circle")
                    Label("点「添加新键盘…」，选「素笺」", systemImage: "2.circle")
                    Label("打字时长按地球键切到素笺", systemImage: "3.circle")
                    Button("打开设置") {
                        if let url = URL(string: UIApplication.openSettingsURLString) {
                            UIApplication.shared.open(url)
                        }
                    }
                }
                // 记忆要完全访问（键盘读 App Group 里的卡片）；和键盘面板同一段话，这里能直接跳到素笺的设置页
                Section {
                    Text(ScopeDisplay.fullAccessExplanation)
                    Text(ScopeDisplay.fullAccessPath)
                        .font(.footnote)
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
                } footer: {
                    Text("与 Mac 版偏好设置是同一份，登录并打开同步后两边互通。")
                }
                Section {
                    TextField("在这里试打", text: $draft, axis: .vertical)
                        .lineLimit(3...8)
                } header: {
                    Text("试一试")
                } footer: {
                    Text("「完全访问」用于按键震动、键盘读你在「记得」里写下的人与事，以及登录后连接服务器（大模型润色、剪贴板与学习数据同步）。不开也能正常打字；没登录时键盘不联网。")
                }
            }
            .navigationTitle("我")
        }
    }
}
