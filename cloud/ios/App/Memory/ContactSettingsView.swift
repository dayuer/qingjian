// 对象设置（02 的 1d）：名字、称呼、这个人的两个提示开关（按人，只对 TA 生效）、导出记忆、忘掉这个人（.alert 确认一次，桥连对象目录一起删）。
// 每一项改了就交给后台写；写的时候界面先按改后的显示（pending），存不上就弹回 store 里的原样并弹原因。
// 「忘掉」等确认框收起后才执行：在按钮回调里直接写，失败提示会撞上确认框的收起动画弹不出来（"already presenting"），变成静默失败。

import SwiftUI

struct ContactSettingsView: View {
    let store: MemoryStore

    let contactId: String

    @State private var name = ""

    @State private var confirmingForget = false

    /// 正在保存的这一版；保存结束后清掉，回到读 store。
    @State private var pending: MemoryContact?

    /// 确认框里点了「忘掉」，等它收起后执行。
    @State private var forgetConfirmed = false

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
        .alert("忘掉\(store.contact(contactId)?.name ?? "")？", isPresented: $confirmingForget) {
            Button("忘掉", role: .destructive) { forgetConfirmed = true }
            Button("再想想", role: .cancel) {}
        } message: {
            Text("\(store.contact(contactId)?.name ?? "")的所有记忆会从这台手机上删除，无法恢复")
        }
        // 成功后首页看到名单变了，会把这个人的详情与设置页一起退掉（MemoryHomeView）
        .onChange(of: confirmingForget) { _, shown in
            guard !shown, forgetConfirmed else { return }
            forgetConfirmed = false
            Task {
                // 等确认框的收起动画走完，失败提示才弹得出来
                try? await Task.sleep(for: .milliseconds(400))
                await store.forget(contactId)
            }
        }
        .navigationTitle("设置")
        .navigationBarTitleDisplayMode(.inline)
        .onAppear { name = store.contact(contactId)?.name ?? "" }
        .onDisappear { saveName() }
    }

    @ViewBuilder
    private func sections(_ contact: MemoryContact) -> some View {
        Section {
            TextField("名字或代号", text: $name)
                .onSubmit { saveName() }
        } header: {
            Text("名字")
        } footer: {
            Text("名字只保存在这台手机上。")
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
                        .font(.caption)
                        .foregroundStyle(.secondary)
                }
            }
            .tint(ColorUsage.appToggle.role.color)
        } footer: {
            Text(store.saving ? MemoryStore.Wording.saving : "只对\(contact.name)生效。")
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

    private func saveName() {
        let trimmed = name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard var next = store.contact(contactId) else { return }
        guard !trimmed.isEmpty else {
            name = next.name
            return
        }
        guard trimmed != next.name else { return }
        next.name = trimmed
        save(next)
    }

    private func save(_ next: MemoryContact) {
        pending = next
        Task {
            if !(await store.saveContact(next)) { name = store.contact(contactId)?.name ?? next.name }
            pending = nil
        }
    }
}
