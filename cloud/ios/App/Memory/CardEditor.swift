// 新建 / 改一张卡（02 的 1c）：写下来（最多 200 字，带计数）、是什么（五个 .opts 胶囊，KindPill）、
// 到哪天（日子与约定才有：一行日期，点开再选；右边的提醒说明只写真有的行为，见 MemoryDetailText.reminderNote）、
// 关键词（一个一个加，最多 8 个、每个 2–8 字，满了「添加」不可点）、删掉这条。上限见 MemoryLimits，桥写入前也会校验。
// 保存在后台做，期间显示「正在保存」、整页置灰；存不上时页面不关、改的内容留着，提示框说原因；存好了但有话要说（冲突已合并）时等用户点掉提示再关。

import SwiftUI

struct CardEditor: View {
    let store: MemoryStore

    let contactId: String

    /// nil 是新建。
    let card: MemoryCard?

    @Environment(\.dismiss) private var dismiss

    @State private var text = ""

    @State private var kind = MemoryCard.Kind.other

    @State private var when = Date()

    @State private var keywords: [String] = []

    @State private var newKeyword = ""

    @State private var closeAfterAlert = false

    /// 「到哪天」那一行点开了日历。
    @State private var pickingDate = false

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    TextField("比如：不吃香菜，喜欢冰美式", text: $text, axis: .vertical)
                        .lineLimit(2...5)
                        .onChange(of: text) { _, value in
                            let clamped = MemoryLimits.clampText(value)
                            if clamped != value { text = clamped }
                        }
                } header: {
                    Text("记忆")
                } footer: {
                    HStack {
                        Spacer()
                        Text(MemoryLimits.counter(text)).monospacedDigit()
                    }
                }
                Section("是什么") {
                    HStack(spacing: 8) {
                        ForEach(KindPill.pills(selected: kind)) { pill in
                            Button {
                                kind = pill.kind
                            } label: {
                                Text(pill.title)
                                    .font(AppFont.font(size: 13))
                                    .foregroundStyle(pill.foreground)
                                    .padding(.horizontal, 13)
                                    .frame(height: 32)
                                    .background(Capsule().fill(pill.background))
                                    .overlay {
                                        if pill.outlined { Capsule().strokeBorder(Color(UIColor.separator)) }
                                    }
                            }
                            .buttonStyle(.plain)
                            .accessibilityAddTraits(pill.selected ? .isSelected : [])
                        }
                    }
                    .listRowBackground(Color.clear)
                    .listRowInsets(EdgeInsets(top: 4, leading: 4, bottom: 4, trailing: 4))
                }
                if kind.hasDate {
                    Section("到哪天") {
                        Button {
                            withAnimation { pickingDate.toggle() }
                        } label: {
                            HStack {
                                Text(MemoryDetailText.dayTitle(when)).font(AppFont.font(size: 15)).foregroundStyle(Theme.ink)
                                Spacer()
                                if let note = MemoryDetailText.reminderNote(kind: kind, contact: store.contact(contactId)) {
                                    Text(note).font(AppFont.font(size: 12)).foregroundStyle(Theme.ink3)
                                }
                            }
                            .contentShape(Rectangle())
                        }
                        .buttonStyle(.plain)
                        if pickingDate {
                            DatePicker("日期", selection: $when, displayedComponents: .date)
                                .datePickerStyle(.graphical)
                                .environment(\.timeZone, MemoryDate.timeZone)
                                .tint(Theme.ink)
                        }
                    }
                }
                Section {
                    ForEach(keywords, id: \.self) { keyword in Text(keyword) }
                        .onDelete { keywords.remove(atOffsets: $0) }
                    HStack {
                        TextField("2–8 个字", text: $newKeyword)
                        Button("添加") {
                            keywords.append(newKeyword.trimmingCharacters(in: .whitespacesAndNewlines))
                            newKeyword = ""
                        }
                        .foregroundStyle(ColorUsage.editorSave.role.color)
                        .disabled(!MemoryLimits.canAdd(newKeyword, to: keywords))
                    }
                } header: {
                    Text("关键词 \(keywords.count) / \(MemoryLimits.maxKeywords)")
                } footer: {
                    Text("打字时出现这些词，键盘会提示这一条；不写就按这条的内容自动找。")
                }
                if let card {
                    Section {
                        Button("删掉这条", role: .destructive) {
                            Task { if await store.deleteCard(card.id, for: contactId) { close() } }
                        }
                        .frame(maxWidth: .infinity)
                    }
                }
            }
            .navigationTitle(card == nil ? "记一条" : "改一条")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("取消") { dismiss() }.foregroundStyle(ColorUsage.editorSave.role.color)
                }
                ToolbarItem(placement: .confirmationAction) {
                    if store.saving {
                        MemorySavingLabel().foregroundStyle(ColorUsage.cardNotice.role.color)
                    } else {
                        Button("存好") { save() }
                            .foregroundStyle(ColorUsage.editorSave.role.color)
                            .disabled(text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                    }
                }
            }
            .disabled(store.saving)
            .onAppear(perform: load)
            .memoryEditorAlert(store)
            // 存好了但有话要说（冲突已合并）：等提示框点掉再关，在按钮回调里直接关会被提示框的收起动画吞掉
            .onChange(of: store.message == nil) { _, cleared in
                if cleared && closeAfterAlert { dismiss() }
            }
        }
    }

    private func load() {
        guard let card else { return }
        text = card.text
        kind = card.kind
        when = card.when.flatMap(MemoryDate.parse) ?? Date()
        keywords = card.keywords
    }

    private func save() {
        var next = card ?? MemoryCard.new(kind: kind, text: "", when: nil, keywords: [])
        next.kind = kind
        next.text = MemoryLimits.clampText(text.trimmingCharacters(in: .whitespacesAndNewlines))
        next.when = kind.hasDate ? MemoryDate.format(when) : nil
        next.keywords = keywords
        next.touchedAt = Int64(Date().timeIntervalSince1970)
        Task { if await store.saveCard(next, for: contactId) { close() } }
    }

    /// 存好了：没话要说就关，有话（冲突已合并）就等提示框点掉再关。
    private func close() {
        if store.message == nil {
            dismiss()
        } else {
            closeAfterAlert = true
        }
    }
}
