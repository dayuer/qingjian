// 开通素笺云服务（05 的 2e 点「了解云服务」之后，或「我 → 素笺云服务」）：一句说明 + 出境同意 + 一个按钮。
// 设计稿没有这一屏（D6），先照系统表单做，交 UI 审计员过。

import SwiftUI

struct CreateSpaceView: View {
    @Bindable var store: SpaceStore

    @Environment(\.dismiss) private var dismiss

    var body: some View {
        Form {
            Section {
                Text(SpaceWording.createIntro)
            }
            Section {
                Toggle(isOn: $store.consented) {
                    Text(SpaceWording.consent).font(AppFont.footnote)
                }
                .toggleStyle(CheckboxToggleStyle())
                Link("了解更多", destination: PrivacyLinks.dataLocation)
                    .font(AppFont.footnote)
            }
            Section {
                Button(SpaceWording.createButton) {
                    Task { await store.create() }
                }
                .disabled(!store.consented || store.busy)
            } footer: {
                if let message = store.message { Text(message) }
            }
            Section {
                NavigationLink(SpaceWording.alreadyHave) { JoinSpaceView(store: store) }
                    .font(AppFont.footnote)
            }
        }
        .navigationTitle(SpaceWording.createTitle)
        .navigationBarTitleDisplayMode(.inline)
        .disabled(store.busy)
        .onChange(of: store.finished) { _, done in
            if done { dismiss() }
        }
    }
}
