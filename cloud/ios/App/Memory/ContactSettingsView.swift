// 对象设置（02 的 1d）：名字、称呼、这个人的两个提示开关（按人，只对 TA 生效）、导出记忆、忘掉这个人（确认一次，桥连对象目录一起删）。
// 每一项改了就写；存不上时开关弹回原样（界面读的是 store 里的），提示框说原因。

import SwiftUI

struct ContactSettingsView: View {
    let store: MemoryStore

    let contactId: String

    @State private var name = ""

    @State private var confirmingForget = false

    var body: some View {
        Form {
            if let contact = store.contact(contactId) {
                Section {
                    TextField("名字或代号", text: $name)
                        .onSubmit { saveName(contact) }
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
                    Toggle("日子提醒", isOn: binding(contact, \.remindOn))
                } footer: {
                    Text("只对\(contact.name)生效。")
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
                .confirmationDialog(
                    "忘掉\(contact.name)？", isPresented: $confirmingForget, titleVisibility: .visible
                ) {
                    // 成功后首页看到名单变了，会把这个人的详情与设置页一起退掉（MemoryHomeView）
                    Button("忘掉", role: .destructive) { store.forget(contactId) }
                    Button("再想想", role: .cancel) {}
                } message: {
                    Text("\(contact.name)的所有记忆会从这台手机上删除，无法恢复")
                }
                .onDisappear { saveName(contact) }
            }
        }
        .navigationTitle("设置")
        .navigationBarTitleDisplayMode(.inline)
        .disabled(!store.canEdit)
        .onAppear { name = store.contact(contactId)?.name ?? "" }
    }

    /// 改一项就写回这个人；读的是 store 里的值，写失败时界面自然回到原样。
    private func binding<Value>(_ contact: MemoryContact, _ field: WritableKeyPath<MemoryContact, Value>) -> Binding<Value> {
        Binding(
            get: { store.contact(contactId)?[keyPath: field] ?? contact[keyPath: field] },
            set: { value in
                guard var next = store.contact(contactId) else { return }
                next[keyPath: field] = value
                store.saveContact(next)
            })
    }

    private func saveName(_ contact: MemoryContact) {
        let trimmed = name.trimmingCharacters(in: .whitespacesAndNewlines)
        guard var next = store.contact(contactId), !trimmed.isEmpty, trimmed != next.name else {
            if trimmed.isEmpty { name = store.contact(contactId)?.name ?? contact.name }
            return
        }
        next.name = trimmed
        if !store.saveContact(next) { name = store.contact(contactId)?.name ?? contact.name }
    }
}
