// 加一个人（05 的 2g / 02 的 3c）。弹层里照设计稿排：大标题「想记得谁？」、名字或代号、「只保存在这台手机上。」、
// 提示里怎么称呼（他 / 她 / TA / 直接用名字，缺省 TA）、可以跳过的几件已知的事、底部大按钮「好了」。
// 几件已知的事都是「点开再填」：空着时右边写「填一下」，点整行才在那一行里变成输入。
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

    /// 「先写几件…」那组里点了「填一下」的那一行；填了内容的那一行不用它、一直显示内容。
    @State private var editingFact: Fact?

    @State private var closeAfterAlert = false

    @FocusState private var nameFocused: Bool

    @FocusState private var focusedFact: Fact?

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 0) {
                HStack {
                    Button("取消") { dismiss() }
                        .font(AppFont.font(size: 15, weight: .semibold))
                        .foregroundStyle(ColorUsage.cardNotice.role.color)
                    Spacer()
                }
                Text("想记得谁？")
                    .font(AppFont.font(size: 26, weight: .semibold))
                    .padding(.top, 20)
                label("名字或代号").padding(.top, 24)
                TextField("", text: $name, prompt: Text("比如：小美").foregroundStyle(Theme.ink3))
                    .font(AppFont.font(size: 16))
                    .focused($nameFocused)
                    .modifier(MemoryFieldStyle())
                    // 内边距在 TextField 外面，点到边上也要能聚焦
                    .contentShape(Rectangle())
                    .onTapGesture { nameFocused = true }
                    .padding(.top, 8)
                Text("只保存在这台手机上。")
                    .font(AppFont.font(size: 12.5))
                    .foregroundStyle(Theme.ink3)
                    .padding(.top, 8)
                label("提示里怎么称呼").padding(.top, 20)
                // 设计稿 05 的 2g / 02 的 3c：系统分段控件（四段等宽、缺省 TA），与对象设置那处一样
                Picker("称呼", selection: $pronoun) {
                    ForEach(MemoryPronoun.choices, id: \.self) { Text($0.title).tag($0) }
                }
                .pickerStyle(.segmented)
                .padding(.top, 8)
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
        // 焦点离开「先写几件…」那一组时，空着的行退回「填一下」
        .onChange(of: focusedFact) { _, now in
            if now == nil, let fact = editingFact { endEditingIfEmpty(fact) }
        }
        .memoryEditorAlert(store)
        // 存好了但有话要说（冲突已合并）：等提示框点掉再关，在按钮回调里直接关会被提示框的收起动画吞掉
        .onChange(of: store.message == nil) { _, cleared in
            if cleared && closeAfterAlert { dismiss() }
        }
    }

    private func label(_ text: String) -> some View {
        Text(text).font(AppFont.font(size: 12, weight: .medium)).foregroundStyle(Theme.ink3)
    }

    /// 设计稿 3c 的 .group：白底圆角、行间细线。几件事都是「点开再填」——空着时右边写「填一下」，
    /// 点整行就在那一行里变成输入（自动聚焦）；填了内容就一直显示内容，清空并失焦退回「填一下」。
    private var facts: some View {
        VStack(spacing: 0) {
            birthdayRow
            rowLine
            factRow("喜欢", text: $likes, prompt: "比如：冰美式", fact: .likes)
            rowLine
            factRow("不喜欢", text: $dislikes, prompt: "比如：香菜", fact: .dislikes)
            rowLine
            extraRow
        }
        .font(AppFont.font(size: 15))
        .padding(.horizontal, 14)
        .background(Color(.secondarySystemGroupedBackground), in: RoundedRectangle(cornerRadius: 14))
        .overlay(RoundedRectangle(cornerRadius: 14).strokeBorder(Hairline.ring))
    }

    private var birthdayRow: some View {
        HStack {
            Text("生日").foregroundStyle(Theme.ink3)
            Spacer(minLength: 12)
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
                fillHint
            }
        }
        .frame(minHeight: 44)
        .contentShape(Rectangle())
        .onTapGesture { if !hasBirthday { hasBirthday = true } }
    }

    private func factRow(_ title: String, text: Binding<String>, prompt: String, fact: Fact) -> some View {
        let showsInput = !text.wrappedValue.isEmpty || editingFact == fact
        return HStack {
            Text(title).foregroundStyle(Theme.ink3)
            Spacer(minLength: 12)
            if showsInput {
                TextField("", text: text, prompt: Text(prompt).foregroundStyle(Theme.ink3))
                    .multilineTextAlignment(.trailing)
                    .focused($focusedFact, equals: fact)
            } else {
                fillHint
            }
        }
        .frame(minHeight: 44)
        .contentShape(Rectangle())
        .onTapGesture {
            guard !showsInput else { return }
            editingFact = fact
            focusedFact = fact
        }
    }

    /// 「+ 再写一条」整行：空着时是灰绿的字，点了这一行变成输入框（行为同以前那个常驻输入框）。
    private var extraRow: some View {
        let showsInput = !extra.isEmpty || editingFact == .extra
        return HStack {
            if showsInput {
                TextField("", text: $extra)
                    .focused($focusedFact, equals: .extra)
                    .frame(maxWidth: .infinity, alignment: .leading)
            } else {
                Text("+ 再写一条").foregroundStyle(Theme.accentInk.color)
                Spacer(minLength: 0)
            }
        }
        .frame(minHeight: 44)
        .contentShape(Rectangle())
        .onTapGesture {
            guard !showsInput else { return }
            editingFact = .extra
            focusedFact = .extra
        }
    }

    /// 空着的那一行右边两个字。
    private var fillHint: some View {
        Text("填一下")
            .font(AppFont.font(size: 12))
            .foregroundStyle(Theme.ink3)
    }

    private var rowLine: some View {
        Rectangle().fill(Hairline.row).frame(height: 1)
    }

    /// 清空并失焦就退回「填一下」；生日那行的「清空」由 × 负责，不在这里管。
    private func endEditingIfEmpty(_ fact: Fact) {
        switch fact {
        case .birthday: break
        case .likes: if trimmed(likes).isEmpty { editingFact = nil }
        case .dislikes: if trimmed(dislikes).isEmpty { editingFact = nil }
        case .extra: if trimmed(extra).isEmpty { editingFact = nil }
        }
    }

    /// 底部大按钮（设计稿 .btn.lg.acc）：灰绿实底、ink 字，保存中换成「正在保存」。
    private var doneButton: some View {
        Button(action: save) {
            Group {
                if store.saving {
                    MemorySavingLabel()
                } else {
                    Text("好了")
                }
            }
            .font(AppFont.font(size: 16, weight: .medium))
            // 设计稿 .btn.acc：灰绿实底、ink 字
            .foregroundStyle(Theme.ink)
            .frame(maxWidth: .infinity, minHeight: 50)
            .background(ColorUsage.addContactDone.role.color, in: Capsule())
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

/// 「先写几件你已经知道的事」那组里的四行。
private enum Fact: Hashable {
    case birthday
    case likes
    case dislikes
    case extra
}
