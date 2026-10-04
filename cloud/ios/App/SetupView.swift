// 主 App 首页：三个 Tab。「记住的」与「本周」是键盘记住的事，「我」是原来的启用步骤、键盘设置、账号与关于、试打框。
// 启动时读一次记忆，从后台回到前台再读（App 在后台期间键盘可能记过一笔）；刚启动的 inactive → active 不算，免得连读两遍。
// 写记忆失败的提示框挂在 TabView 上，弹出的编辑页各自另挂一个。

import SwiftUI
import UIKit

struct SetupView: View {
    @Environment(\.scenePhase) private var scenePhase

    @State private var draft = ""

    @State private var store = SettingsStore()

    @State private var account = AccountStore()

    @State private var memory = MemoryStore()

    @State private var wasInBackground = false

    var body: some View {
        TabView {
            MemoryHomeView(store: memory)
                .tabItem { Label("记住的", systemImage: "heart.text.square") }
            WeekView(store: memory)
                .tabItem { Label("本周", systemImage: "calendar") }
            me
                .tabItem { Label("我", systemImage: "person.crop.circle") }
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

    private var me: some View {
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
                Section {
                    if store.available {
                        NavigationLink("键盘设置") { KeyboardSettingsView(store: store) }
                        NavigationLink("账号") { AccountView(store: account) }
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
                    Text("「完全访问」用于按键震动、键盘读你在「记住的」里写下的人与事，以及登录后连接服务器（大模型润色、剪贴板与学习数据同步）。不开也能正常打字；没登录时键盘不联网。")
                }
            }
            .navigationTitle("我")
        }
    }
}
