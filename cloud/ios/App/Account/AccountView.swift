// 账号页：没登录时 Apple 与邮箱两种登录；登录后显示登录方式、设备、四个功能开关、退出登录与删除账号。
// 开关改的是服务器上的同意记录，成功后桥同时写回 cloud.toml，键盘下次弹出时按新开关连。

import SwiftUI

struct AccountView: View {
    let store: AccountStore

    @State private var confirmingSignOut = false

    @State private var confirmingDelete = false

    var body: some View {
        Form {
            if let message = store.message {
                Section { Text(message).foregroundStyle(.secondary) }
            }
            if let state = store.state {
                if state.signedIn {
                    signedIn(state)
                } else {
                    SignInSections(store: store)
                }
                Section {
                    LabeledContent("服务器", value: state.server)
                } footer: {
                    Text("没登录时键盘完全离线。登录后的功能要在系统设置里给素笺打开「允许完全访问」才能联网。")
                }
            } else {
                ProgressView()
            }
        }
        .navigationTitle("账号")
        .disabled(store.busy)
        .overlay {
            if store.busy { ProgressView() }
        }
        .task { await store.refresh() }
    }

    @ViewBuilder
    private func signedIn(_ state: AccountState) -> some View {
        Section("登录方式") {
            ForEach(state.identities, id: \.self) { identity in
                LabeledContent(identity.title, value: identity.label ?? "")
            }
        }
        Section {
            ForEach(CloudFeature.allCases) { feature in
                Toggle(feature.title, isOn: Binding(
                    get: { store.state?.consents[feature] ?? false },
                    set: { value in Task { await store.setConsent(feature, value) } }))
                .tint(ColorUsage.appToggle.role.color)
            }
        } header: {
            Text("功能")
        } footer: {
            Text("都默认关闭。关掉某项会同时删除服务器上这部分数据，本机数据不受影响。")
        }
        Section("设备") {
            ForEach(state.sessions) { device in
                DeviceRow(device: device) { Task { await store.revoke(device) } }
            }
        }
        Section {
            Button("退出登录") { confirmingSignOut = true }
            Button("删除账号", role: .destructive) { confirmingDelete = true }
        }
        .confirmationDialog("退出登录？", isPresented: $confirmingSignOut, titleVisibility: .visible) {
            Button("退出登录", role: .destructive) { Task { await store.signOut() } }
        } message: {
            Text("退出后键盘不再连服务器，本机的输入习惯保留。")
        }
        .confirmationDialog("删除账号？", isPresented: $confirmingDelete, titleVisibility: .visible) {
            Button("永久删除账号", role: .destructive) { Task { await store.deleteAccount() } }
        } message: {
            Text("会立即删除服务器上这个账号的全部数据（剪贴板、学习数据、设置、输入日志）、撤销 Apple 授权并让所有设备退出登录，无法恢复。本机的输入习惯不受影响。")
        }
    }
}
