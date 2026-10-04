// 加一个人（05 的 2g），弹层里照设计稿排：衬线大标题「想记得谁？」、名字或代号、「只保存在这台手机上。」、
// 提示里怎么称呼（他 / 她 / TA / 直接用名字，缺省 TA）、可以跳过的几件已知的事、底部大按钮「好了」。
// 生日就叫「生日」，日子按年重复，填出生日期也会每年提醒。保存在后台做，期间按钮换成「正在保存」；存不上时弹层不关、填的留着。

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

    @FocusState private var nameFocused: Bool

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 0) {
                HStack {
                    Button("取消") { dismiss() }
                        .font(.system(size: 15))
                        .foregroundStyle(ColorUsage.cardNotice.role.color)
                    Spacer()
                }
                Text("想记得谁？")
                    .font(.system(size: 26, weight: .semibold, design: .serif))
                    .padding(.top, 20)
                label("名字或代号").padding(.top, 24)
                TextField("", text: $name)
                    .font(.system(size: 16))
                    .focused($nameFocused)
                    .modifier(MemoryFieldStyle())
                    // 内边距在 TextField 外面，点到边上也要能聚焦
                    .contentShape(Rectangle())
                    .onTapGesture { nameFocused = true }
                    .padding(.top, 8)
                Text("只保存在这台手机上。")
                    .font(.system(size: 12.5))
                    .foregroundStyle(.secondary)
                    .padding(.top, 8)
                label("提示里怎么称呼").padding(.top, 20)
                MemoryPronounPicker(selection: $pronoun).padding(.top, 8)
                label("先写几件你已经知道的事（可以跳过）").padding(.top, 22)
                facts.padding(.top, 8)
            }
            .padding(.horizontal, 24)
            .padding(.top, 16)
            .padding(.bottom, 24)
        }
        .scrollDismissesKeyboard(.interactively)
        .safeAreaInset(edge: .bottom) { doneButton }
        .disabled(store.saving)
        .memoryEditorAlert(store)
        // 存好了但有话要说（冲突已合并）：等提示框点掉再关，在按钮回调里直接关会被提示框的收起动画吞掉
        .onChange(of: store.message == nil) { _, cleared in
            if cleared && closeAfterAlert { dismiss() }
        }
    }

    private func label(_ text: String) -> some View {
        Text(text).font(.system(size: 12, weight: .medium)).foregroundStyle(.secondary)
    }

    /// 设计稿的 .group：白底圆角、行间细线，左边灰字标题，右边填的内容。
    private var facts: some View {
        VStack(spacing: 0) {
            HStack {
                Text("生日").foregroundStyle(.secondary)
                Spacer()
                if hasBirthday {
                    DatePicker("生日", selection: $birthday, displayedComponents: .date)
                        .labelsHidden()
                        .environment(\.timeZone, MemoryDate.timeZone)
                    Button {
                        hasBirthday = false
                    } label: {
                        Image(systemName: "xmark.circle.fill").foregroundStyle(Color(.tertiaryLabel))
                    }
                    .accessibilityLabel("不填生日")
                } else {
                    Button("填一下") { hasBirthday = true }
                        .foregroundStyle(ColorUsage.editorSave.role.color)
                }
            }
            .frame(minHeight: 44)
            Divider()
            factRow("喜欢", text: $likes, prompt: "比如：冰美式")
            Divider()
            factRow("不喜欢", text: $dislikes, prompt: "比如：香菜")
            Divider()
            TextField("+ 再写一条", text: $extra)
                .frame(minHeight: 44)
        }
        .font(.system(size: 15))
        .padding(.horizontal, 14)
        .background(Color(.secondarySystemGroupedBackground), in: RoundedRectangle(cornerRadius: 14))
        .overlay(RoundedRectangle(cornerRadius: 14).strokeBorder(Color(.separator).opacity(0.5)))
    }

    private func factRow(_ title: String, text: Binding<String>, prompt: String) -> some View {
        HStack {
            Text(title).foregroundStyle(.secondary)
            TextField(prompt, text: text).multilineTextAlignment(.trailing)
        }
        .frame(minHeight: 44)
    }

    /// 底部大按钮（设计稿 .btn.lg）：中性色实底，保存中换成「正在保存」。
    private var doneButton: some View {
        Button(action: save) {
            Group {
                if store.saving {
                    MemorySavingLabel()
                } else {
                    Text("好了")
                }
            }
            .font(.system(size: 16, weight: .medium))
            .foregroundStyle(Color(.systemBackground))
            .frame(maxWidth: .infinity, minHeight: 50)
            .background(ColorUsage.editorSave.role.color, in: Capsule())
        }
        .disabled(trimmed(name).isEmpty || store.saving)
        .opacity(trimmed(name).isEmpty ? 0.35 : 1)
        .padding(.horizontal, 24)
        .padding(.bottom, 12)
        .background(Color(.systemBackground))
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
        Task {
            guard await store.addContact(contact, cards: cards) else { return }
            if store.message == nil {
                dismiss()
            } else {
                closeAfterAlert = true
            }
        }
    }
}
