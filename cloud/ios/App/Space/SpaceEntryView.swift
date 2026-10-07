// 「我 → 素笺云服务」：没开通就开通；开通了显示状态与云功能开关（同意页见 Account/ConsentSheet）。

import SwiftUI

struct SpaceEntryView: View {
    @Bindable var store: SpaceStore

    /// 云功能开关走账号那套桥调用（同一个 cloud.toml）；这里只是个能点到的入口。
    @State private var account = AccountStore()

    /// 等用户过同意说明的功能（记忆 / 输入日志）：非 nil 时弹同意页。
    @State private var pendingConsent: CloudFeature?

    var body: some View {
        Group {
            if store.signedIn == true {
                List {
                    Section {
                        Text(SpaceWording.openedNote)
                    } footer: {
                        Text(SpaceWording.openedMore).font(AppFont.footnote)
                    }
                    Section {
                        ForEach(CloudFeature.allCases) { feature in
                            Toggle(feature.title, isOn: Binding(
                                get: { account.state?.consents[feature] ?? false },
                                set: { value in
                                    // 记忆与输入日志要先过一遍同意说明（写明供应商与数据流向），用户点头才开
                                    if value, ConsentCopy.of(feature) != nil {
                                        pendingConsent = feature
                                    } else {
                                        Task { await account.setConsent(feature, value) }
                                    }
                                }))
                                .tint(ColorUsage.appToggle.role.color)
                        }
                    } header: {
                        Text("云功能")
                    } footer: {
                        Text("都默认关闭。关掉某项会同时删除服务器上这部分数据，本机数据不受影响。")
                    }
                    Section {
                        // 同一空间里的设备都列出来：本机标一下，别的设备可以在这里解绑
                        ForEach(account.state?.sessions ?? []) { device in
                            DeviceRow(device: device) {
                                Task { await account.revoke(device) }
                            }
                        }
                        if let code = store.pairCode {
                            LabeledContent("匹配码", value: code.pairCode)
                            Text(SpaceWording.addDeviceIntro)
                                .font(AppFont.footnote)
                                .foregroundStyle(.secondary)
                            Button(SpaceWording.addDeviceStop) { store.stopAddingDevice() }
                        } else {
                            Button(SpaceWording.addDevice) {
                                Task { await store.addDevice() }
                            }
                        }
                        ForEach(store.requests, id: \.id) { request in
                            VStack(alignment: .leading, spacing: 6) {
                                Text(String(format: SpaceWording.requestTitle, request.name))
                                HStack {
                                    Button(SpaceWording.allow) {
                                        // 允许之后那台设备就在列表里了，重新取一遍
                                        Task {
                                            await store.decide(request, allow: true)
                                            await account.refresh()
                                        }
                                    }
                                    .buttonStyle(.borderedProminent)
                                    Button(SpaceWording.deny, role: .destructive) {
                                        Task { await store.decide(request, allow: false) }
                                    }
                                }
                            }
                        }
                    } header: {
                        Text("设备")
                    } footer: {
                        // 解绑的结果（成功或失败的原因）在这里说一句
                        if let message = account.message {
                            Text(message).font(AppFont.footnote)
                        } else {
                            Text(SpaceWording.devicesMore).font(AppFont.footnote)
                        }
                    }
                    Section {
                        Button("清空云端输入记录", role: .destructive) {
                            Task {
                                if await account.clearInputLog() {
                                    account.message = "云端输入记录已清空"
                                }
                            }
                        }
                    } header: {
                        Text("数据")
                    } footer: {
                        Text("清空服务器上已上传的全部输入记录，本机日志一并删除。")
                    }
                }
                .navigationTitle(SpaceWording.entryTitle)
                .navigationBarTitleDisplayMode(.inline)
                .sheet(item: $pendingConsent) { feature in
                    ConsentSheet(feature: feature) {
                        Task { await account.setConsent(feature, true) }
                    }
                }
            } else {
                CreateSpaceView(store: store)
            }
        }
        .onAppear { store.refresh() }
        .onDisappear { store.stopAddingDevice() }
        .task { await account.refresh() }
    }
}
