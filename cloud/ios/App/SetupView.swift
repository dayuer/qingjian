// 主 App 首页：启用步骤、设置与账号入口、试打框。

import SwiftUI
import UIKit

struct SetupView: View {
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
                Section {
                    if store.available {
                        NavigationLink("键盘设置") { KeyboardSettingsView(store: store) }
                        NavigationLink("账号") { AccountView(store: account) }
                    } else {
                        Text("这个安装包没有开通 App Group，设置改不到键盘上。").foregroundStyle(.secondary)
                    }
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
                    Text("「完全访问」用于按键震动，以及登录后连接服务器（大模型润色、剪贴板与学习数据同步）。不开也能正常打字；没登录时键盘不联网。")
                }
            }
            .navigationTitle("素笺")
        }
    }
}
