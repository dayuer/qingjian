// 加一个人（05 的 2g）：名字或代号、提示里怎么称呼（他 / 她 / TA / 直接用名字，缺省 TA）、可以跳过的几件已知的事。
// 生日就叫「生日」，日子按年重复，填出生日期也会每年提醒。存不上时页面不关、填的留着。

import SwiftUI

struct ContactEditor: View {
    let store: MemoryStore

    @Environment(\.dismiss) private var dismiss

    @State private var name = ""

    @State private var pronoun = MemoryPronoun.ta

    @State private var hasBirthday = false

    @State private var birthday = Date()

    @State private var likes = ""

    @State private var dislikes = ""

    @State private var extra = ""

    @State private var closeAfterAlert = false

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    TextField("名字或代号", text: $name)
                } header: {
                    Text("名字或代号")
                } footer: {
                    Text("只保存在这台手机上。")
                }
                Section("提示里怎么称呼") {
                    Picker("称呼", selection: $pronoun) {
                        ForEach(MemoryPronoun.choices, id: \.self) { Text($0.title).tag($0) }
                    }
                    .pickerStyle(.segmented)
                }
                Section {
                    Toggle("生日", isOn: $hasBirthday)
                    if hasBirthday {
                        DatePicker("日期", selection: $birthday, displayedComponents: .date)
                            .environment(\.timeZone, MemoryDate.timeZone)
                    }
                    TextField("喜欢", text: $likes)
                    TextField("不喜欢", text: $dislikes)
                    TextField("再写一条", text: $extra)
                } header: {
                    Text("先写几件你已经知道的事（可以跳过）")
                }
            }
            .navigationTitle("想记得谁？")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("取消") { dismiss() }.foregroundStyle(ColorUsage.editorSave.role.color)
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("好了") { save() }
                        .foregroundStyle(ColorUsage.editorSave.role.color)
                        .disabled(trimmed(name).isEmpty)
                }
            }
            .memoryEditorAlert(store)
            // 存好了但有话要说（冲突已合并）：等提示框点掉再关，在按钮回调里直接关会被提示框的收起动画吞掉
            .onChange(of: store.message == nil) { _, cleared in
                if cleared && closeAfterAlert { dismiss() }
            }
        }
    }

    private func trimmed(_ text: String) -> String {
        text.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private func save() {
        let contact = MemoryContact.new(name: trimmed(name), pronoun: pronoun)
        var cards: [MemoryCard] = []
        if hasBirthday {
            cards.append(.new(kind: .date, text: "生日", when: MemoryDate.format(birthday), keywords: ["生日"]))
        }
        if !trimmed(likes).isEmpty {
            cards.append(.new(kind: .preference, text: MemoryLimits.clampText("喜欢\(trimmed(likes))"), when: nil, keywords: []))
        }
        if !trimmed(dislikes).isEmpty {
            cards.append(
                .new(kind: .preference, text: MemoryLimits.clampText("不喜欢\(trimmed(dislikes))"), when: nil, keywords: []))
        }
        if !trimmed(extra).isEmpty {
            cards.append(.new(kind: .other, text: MemoryLimits.clampText(trimmed(extra)), when: nil, keywords: []))
        }
        guard store.addContact(contact, cards: cards) else { return }
        if store.message == nil {
            dismiss()
        } else {
            closeAfterAlert = true
        }
    }
}
