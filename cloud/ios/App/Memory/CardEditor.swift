// 新建 / 改一张卡（02 的 1c）：盖在详情页上的底部弹层，纸底、自绘表单。
// 写下来（最多 200 字，带计数）、是什么（五个 .opts 胶囊，KindPill）、到哪天（日子与约定才有：一行日期，
// 点开在同一组里出日历；右边的提醒说明只写真有的行为，见 MemoryDetailText.reminderNote）、关键词（KeywordRows）、删掉这条。
// 上限见 MemoryLimits，桥写入前也会校验。保存在后台做，期间整页置灰；存不上时页面不关、改的内容留着，提示框说原因。

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

    @State private var closeAfterAlert = false

    /// 「到哪天」那一行点开了日历。
    @State private var pickingDate = false

    var body: some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 10) {
                topRow
                PaperLabel(text: "记忆")
                TextField("比如：不吃香菜，喜欢冰美式", text: $text, axis: .vertical)
                    .lineLimit(2...5)
                    .modifier(PaperFieldStyle(vertical: 10))
                    .onChange(of: text) { _, value in
                        let clamped = MemoryLimits.clampText(value)
                        if clamped != value { text = clamped }
                    }
                Text(MemoryLimits.counter(text))
                    .font(AppFont.font(size: 11.5))
                    .foregroundStyle(Theme.ink3)
                    .monospacedDigit()
                    .frame(maxWidth: .infinity, alignment: .trailing)
                    .padding(.top, -6)
                PaperLabel(text: "是什么")
                kindPills
                if kind.hasDate {
                    PaperLabel(text: "到哪天")
                    dateGroup
                }
                KeywordRows(keywords: $keywords)
                if let card {
                    Button("删掉这条") {
                        Task { if await store.deleteCard(card.id, for: contactId) { close() } }
                    }
                    .font(AppFont.font(size: 13, weight: .medium))
                    .foregroundStyle(Theme.ink)
                    .frame(maxWidth: .infinity)
                    .padding(.top, 8)
                }
            }
            .padding(.horizontal, 20)
            // iOS 26 的大档弹层上沿比设计稿的 top:56 高约 9pt，拖动条又是系统画的、不占内容的位置，
            // 所以额外补 29pt 让顶端那一行落在稿子的 y≈90（叠图实测）。换机型或系统版本要重新量。
            .padding(.top, 39)
            .padding(.bottom, 40)
        }
        .background(Theme.paper)
        .disabled(store.saving)
        .onAppear(perform: load)
        .memoryEditorAlert(store)
        // 存好了但有话要说（冲突已合并）：等提示框点掉再关，在按钮回调里直接关会被提示框的收起动画吞掉
        .onChange(of: store.message == nil) { _, cleared in
            if cleared && closeAfterAlert { dismiss() }
        }
    }

    // MARK: - 顶部一行

    private var topRow: some View {
        HStack(spacing: 12) {
            Button("取消") { dismiss() }
                .font(AppFont.font(size: 15))
                .foregroundStyle(ColorUsage.editorCancel.role.color)
            Spacer(minLength: 0)
            Text(card == nil ? "记一条" : "改一条")
                .font(AppFont.font(size: 16, weight: .semibold))
                .foregroundStyle(Theme.ink)
            Spacer(minLength: 0)
            if store.saving {
                MemorySavingLabel()
                    .font(AppFont.font(size: 15))
                    .foregroundStyle(ColorUsage.cardNotice.role.color)
            } else {
                Button("存好") { save() }
                    .font(AppFont.font(size: 15, weight: .medium))
                    .foregroundStyle(ColorUsage.editorConfirm.role.color)
                    .disabled(text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
                    .opacity(text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? 0.4 : 1)
            }
        }
    }

    // MARK: - 是什么、到哪天

    private var kindPills: some View {
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
                            if pill.outlined { Capsule().strokeBorder(Hairline.line) }
                        }
                }
                .buttonStyle(.plain)
                .accessibilityAddTraits(pill.selected ? .isSelected : [])
            }
        }
    }

    /// 设计稿 1c：「到哪天」一行右边是提前几天提醒，点这一行在同一组里展开日历。
    private var dateGroup: some View {
        PaperGroup(inset: 0) {
            Button {
                withAnimation { pickingDate.toggle() }
            } label: {
                PaperRow(
                    title: MemoryDetailText.dayTitle(when),
                    side: MemoryDetailText.reminderNote(kind: kind, contact: store.contact(contactId)),
                    tight: true)
            }
            .buttonStyle(.plain)
            if pickingDate {
                PaperRowLine()
                DatePicker("日期", selection: $when, displayedComponents: .date)
                    .datePickerStyle(.graphical)
                    .environment(\.timeZone, MemoryDate.timeZone)
                    .tint(Theme.ink)
                    .padding(.horizontal, 6)
            }
        }
    }

    // MARK: - 数据

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
