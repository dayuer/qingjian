// 在新设备上加入已有的空间：输一张 8 位匹配码 → 等旧设备允许 → 加入好了。
// 找回方式（Apple / 邮箱 / 微信）按 D7 往后放，这一页只有输码。设计稿没画（D6），照系统表单做。

import SwiftUI

struct JoinSpaceView: View {
    @Bindable var store: SpaceStore

    @Environment(\.dismiss) private var dismiss

    @State private var code = ""

    @FocusState private var focused: Bool

    var body: some View {
        Form {
            Section {
                Text(SpaceWording.joinIntro).font(AppFont.footnote)
            }
            if store.waiting {
                Section {
                    HStack(spacing: 10) {
                        ProgressView()
                        Text(SpaceWording.waiting)
                    }
                    Text(SpaceWording.waitingHint)
                        .font(AppFont.footnote)
                        .foregroundStyle(.secondary)
                    Button(SpaceWording.cancel) { store.cancelWaiting() }
                }
            } else {
                Section {
                    TextField(SpaceWording.joinField, text: $code)
                        .textInputAutocapitalization(.characters)
                        .autocorrectionDisabled()
                        .focused($focused)
                    Button(SpaceWording.joinButton) {
                        Task { await store.join(code: code) }
                    }
                    .disabled(!MatchCode.isComplete(code) || store.busy)
                } footer: {
                    Text(SpaceWording.joinHint).font(AppFont.footnote)
                }
            }
            if let message = store.message {
                Section { Text(message).foregroundStyle(.secondary) }
            }
        }
        .navigationTitle(SpaceWording.joinTitle)
        .navigationBarTitleDisplayMode(.inline)
        .onAppear { store.clearMessage() }
        .onChange(of: store.finished) { _, done in
            if done { dismiss() }
        }
    }
}
