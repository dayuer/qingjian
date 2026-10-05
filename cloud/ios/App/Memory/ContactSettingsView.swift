// 对象设置（02 的 1d）：名字与「在键盘上显示为」（代号，空着就显示名字）、所在场景（场景只是分组，可以换）、
// 置顶（键盘的选择面板先摆置顶的人，一个场景最多 4 个）、称呼、这个人的两个提示开关（按人，只对 TA 生效）、
// 导出记忆、忘掉这个人（底部弹层确认一次，桥连对象目录一起删）。
// 每一项改了就交给后台写；写的时候界面先按改后的显示（pending），存不上就弹回 store 里的原样并弹原因。
// 「忘掉」等弹层收起后（.sheet 的 onDismiss）才执行：在按钮回调里直接写，失败提示会撞上弹层的收起动画弹不出来（"already presenting"），变成静默失败。

import SwiftUI

struct ContactSettingsView: View {
    let store: MemoryStore

    let contactId: String

    @State private var name = ""

    @State private var displayName = ""

    @State private var confirmingForget = false

    /// 弹层里点了「忘掉」，等它收起后执行。
    @State private var forgetChosen = false

    /// 正在保存的这一版；保存结束后清掉，回到读 store。
    @State private var pending: MemoryContact?

    var body: some View {
        Form {
            if let error = store.loadError {
                MemoryFailureBanner(text: error) { Task { await store.reload() } }
            }
            if let contact = pending ?? store.contact(contactId) {
                Group { sections(contact) }
                    .disabled(!store.canEdit || store.saving)
            }
        }
        .sheet(isPresented: $confirmingForget, onDismiss: forgetIfChosen) {
            if let contact = store.contact(contactId) {
                ForgetContactSheet(
                    name: contact.name, knownDays: contact.knownDays(), cardCount: store.cards(of: contactId).count
                ) { forgetChosen = true }
            }
        }
        // 成功后首页看到名单变了，会把这个人的详情与设置页一起退掉（RememberView）
        .navigationTitle("设置")
        .navigationBarTitleDisplayMode(.inline)
        .onAppear {
            name = store.contact(contactId)?.name ?? ""
            displayName = store.contact(contactId)?.displayName ?? ""
        }
        .onDisappear { saveTexts() }
    }

    @ViewBuilder
    private func sections(_ contact: MemoryContact) -> some View {
        Section {
            LabeledContent("名字") {
                TextField("名字", text: $name)
                    .multilineTextAlignment(.trailing)
                    .onSubmit { saveTexts() }
            }
            LabeledContent("在键盘上显示为") {
                TextField(name.isEmpty ? contact.name : name, text: $displayName)
                    .multilineTextAlignment(.trailing)
                    .onSubmit { saveTexts() }
                    .onChange(of: displayName) { _, value in
                        let clamped = String(value.prefix(MemoryDetailText.maxDisplayNameChars))
                        if clamped != value { displayName = clamped }
                    }
            }
        } footer: {
            Text("名字只保存在这台手机上。键盘上可以改用代号。")
        }
        Section {
            Picker("所在场景", selection: binding(contact, \.scene)) {
                ForEach(store.scenes) { scene in
                    Text(scene.name).tag(scene.id)
                }
            }
            Toggle(isOn: pinBinding(contact)) {
                VStack(alignment: .leading, spacing: 3) {
                    Text("置顶")
                    Text(MemoryStore.Wording.pinNote(store.pinnedCount(in: contact.scene)))
                        .font(AppFont.caption)
                        .foregroundStyle(.secondary)
                }
            }
            .tint(ColorUsage.appToggle.role.color)
        }
        Section("提示里怎么称呼") {
            Picker("称呼", selection: binding(contact, \.pronoun)) {
                ForEach(MemoryPronoun.choices, id: \.self) { Text($0.title).tag($0) }
            }
            .pickerStyle(.segmented)
        }
        Section {
            Toggle("打字时提示", isOn: binding(contact, \.hintOn))
                .tint(ColorUsage.appToggle.role.color)
            Toggle(isOn: binding(contact, \.remindOn)) {
                VStack(alignment: .leading, spacing: 3) {
                    Text("日子提醒")
                    Text(MemoryStore.Wording.remindOffNote(contact))
                        .font(AppFont.caption)
                        .foregroundStyle(.secondary)
                }
            }
            .tint(ColorUsage.appToggle.role.color)
        } footer: {
            Text(store.saving ? MemoryStore.Wording.saving : MemoryStore.Wording.switchesNote(contact))
        }
        Section {
            ShareLink(item: store.exportText(contactId)) {
                Label("导出记忆", systemImage: "square.and.arrow.up")
            }
            .foregroundStyle(ColorUsage.contactSettingsButton.role.color)
        }
        Section {
            Button("忘掉这个人", role: .destructive) { confirmingForget = true }
                .frame(maxWidth: .infinity)
        }
    }

    /// 置顶：勾上就是现在置顶（记时间，早置顶的排在前面），取消就是没置顶。
    /// 一个场景最多几个由桥兜底（超了返回 pin_limit），这里先按本地数出来的人数把开关关掉，少一次来回。
    private func pinBinding(_ contact: MemoryContact) -> Binding<Bool> {
        Binding(
            get: { (pending ?? store.contact(contactId) ?? contact).pinnedAt != nil },
            set: { pinned in
                guard var next = store.contact(contactId) else { return }
                if pinned {
                    guard store.pinnedCount(in: next.scene) < MemoryScene.maxPinned else {
                        store.message = MemoryStore.Wording.pinLimit()
                        return
                    }
                    next.pinnedAt = Int64(Date().timeIntervalSince1970)
                } else {
                    next.pinnedAt = nil
                }
                save(next)
            })
    }

    /// 改一项就写回这个人。
    private func binding<Value>(
        _ contact: MemoryContact, _ field: WritableKeyPath<MemoryContact, Value>
    ) -> Binding<Value> {
        Binding(
            get: { (pending ?? store.contact(contactId) ?? contact)[keyPath: field] },
            set: { value in
                guard var next = store.contact(contactId) else { return }
                next[keyPath: field] = value
                save(next)
            })
    }

    /// 名字与代号一起收拾、一起写：分两次写的话，第二次读到的还是旧名字，会把第一次改的盖掉。
    /// 名字空着就退回原来的；代号空着或和名字一样就不存（键盘上显示名字）。
    private func saveTexts() {
        guard var next = store.contact(contactId) else { return }
        let trimmed = name.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty {
            name = next.name
        } else {
            next.name = trimmed
        }
        next.displayName = MemoryDetailText.displayName(displayName, name: next.name)
        guard next != store.contact(contactId) else { return }
        save(next)
    }

    private func forgetIfChosen() {
        guard forgetChosen else { return }
        forgetChosen = false
        Task { await store.forget(contactId) }
    }

    private func save(_ next: MemoryContact) {
        pending = next
        Task {
            if !(await store.saveContact(next)) {
                name = store.contact(contactId)?.name ?? next.name
                displayName = store.contact(contactId)?.displayName ?? ""
            }
            pending = nil
        }
    }
}
