// 对象设置（02 的 1d）：名字与「在键盘上显示为」（代号，空着就显示名字）、置顶（键盘上先摆置顶的人，全局最多 4 个）、
// 称呼、这个人的两个提示开关（按人，只对 TA 生效）、改写用哪个技能、导出记忆、忘掉这个人（底部弹层确认一次，桥连对象目录一起删）。
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
        PaperPage(spacing: 8) {
            if let error = store.loadError {
                MemoryFailureBanner(text: error) { Task { await store.reload() } }
                    .padding(.horizontal, 20)
            }
            if let contact = pending ?? store.contact(contactId) {
                sections(contact)
                    .disabled(!store.canEdit || store.saving)
            }
        }
        .navigationTitle("设置")
        .navigationBarTitleDisplayMode(.inline)
        .sheet(isPresented: $confirmingForget, onDismiss: forgetIfChosen) {
            if let contact = store.contact(contactId) {
                ForgetContactSheet(
                    name: contact.name, knownDays: contact.knownDays(), cardCount: store.cards(of: contactId).count
                ) { forgetChosen = true }
            }
        }
        // 成功后首页看到名单变了，会把这个人的详情与设置页一起退掉（RememberView）
        .onAppear {
            name = store.contact(contactId)?.name ?? ""
            displayName = store.contact(contactId)?.displayName ?? ""
        }
        .onDisappear { saveTexts() }
    }

    @ViewBuilder
    private func sections(_ contact: MemoryContact) -> some View {
        PaperGroup {
            textRow("名字", text: $name, placeholder: "名字")
            PaperRowLine()
            textRow(
                "在键盘上显示为", text: $displayName, placeholder: name.isEmpty ? contact.name : name,
                clampTo: MemoryDetailText.maxDisplayNameChars)
        }
        PaperLead(text: "名字只保存在这台手机上。键盘上可以改用代号。", small: true)

        PaperGroup {
            PaperRow(title: "置顶", tight: true) {
                Toggle("", isOn: pinBinding(contact)).labelsHidden().tint(ColorUsage.appToggle.role.color)
            }
        }
        PaperLead(text: MemoryStore.Wording.pinNote(store.pinnedCount), small: true)

        PaperGroup {
            VStack(alignment: .leading, spacing: 8) {
                Text("提示里怎么称呼").font(AppFont.font(size: 15))
                Picker("称呼", selection: binding(contact, \.pronoun)) {
                    ForEach(MemoryPronoun.choices, id: \.self) { Text($0.title).tag($0) }
                }
                .pickerStyle(.segmented)
            }
            .padding(.horizontal, 14)
            .padding(.vertical, 10)
        }

        PaperGroup {
            PaperRow(title: "打字时提示", tight: true) {
                Toggle("", isOn: binding(contact, \.hintOn)).labelsHidden().tint(ColorUsage.appToggle.role.color)
            }
            PaperRowLine()
            PaperRow(title: "日子提醒", subtitle: MemoryStore.Wording.remindOffNote(contact), tight: true) {
                Toggle("", isOn: binding(contact, \.remindOn)).labelsHidden().tint(ColorUsage.appToggle.role.color)
            }
        }
        PaperLead(text: store.saving ? MemoryStore.Wording.saving : MemoryStore.Wording.switchesNote(contact), small: true)

        if !store.skills.isEmpty {
            PaperGroup {
                PaperRow(title: "改写用哪个技能", tight: true) {
                    Picker("改写用哪个技能", selection: skillBinding(contact)) {
                        Text("用默认").tag(String?.none)
                        ForEach(store.skills) { Text($0.name).tag(String?.some($0.id)) }
                    }
                    .labelsHidden()
                    .pickerStyle(.menu)
                }
            }
            PaperLead(text: "在键盘上按一下就能改；这里选的是跟这个人说话时默认用哪个。", small: true)
        }

        PaperGroup {
            ShareLink(item: store.exportText(contactId)) {
                PaperRow(title: "导出记忆", chevron: true, tight: true)
            }
            .buttonStyle(.plain)
            PaperRowLine()
            // 新稿把这一处改回系统红：危险操作要看得出来（确认仍在底部弹层里）
            Button { confirmingForget = true } label: {
                PaperRow(title: "忘掉这个人", tight: true, titleColor: Theme.danger)
            }
            .buttonStyle(.plain)
        }
    }

    /// 「名字」那种一行：标题在左，输入框占右边、右对齐；`clampTo` 是代号那样要卡住字数的。
    private func textRow(
        _ title: String, text: Binding<String>, placeholder: String, clampTo: Int? = nil
    ) -> some View {
        PaperRow(title: title, tight: true) {
            TextField(placeholder, text: text)
                .font(AppFont.font(size: 15))
                .foregroundStyle(Theme.ink)
                .multilineTextAlignment(.trailing)
                .onSubmit { saveTexts() }
                .onChange(of: text.wrappedValue) { _, value in
                    guard let clampTo else { return }
                    let clamped = String(value.prefix(clampTo))
                    if clamped != value { text.wrappedValue = clamped }
                }
        }
    }

    /// 置顶：勾上就是现在置顶（记时间，早置顶的排在前面），取消就是没置顶。
    /// 最多几个由桥兜底（超了返回 pin_limit），这里先按本地数出来的人数把开关关掉，少一次来回。
    private func pinBinding(_ contact: MemoryContact) -> Binding<Bool> {
        Binding(
            get: { (pending ?? store.contact(contactId) ?? contact).pinnedAt != nil },
            set: { pinned in
                guard var next = store.contact(contactId) else { return }
                if pinned {
                    guard store.pinnedCount < MemoryStore.pinLimit else {
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

    /// 改写技能：nil 是「用默认」（设置里那个）；选了就写回这个人。
    private func skillBinding(_ contact: MemoryContact) -> Binding<String?> {
        Binding(
            get: { (pending ?? store.contact(contactId) ?? contact).skill },
            set: { value in
                guard var next = store.contact(contactId) else { return }
                next.skill = value
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
