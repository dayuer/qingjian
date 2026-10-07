// 对象详情（02 的 1b）：56pt 头像、名字、「认识 n 天 · n 条记忆」，卡片按「日子与约定」与其余种类分组。
// 有日子的卡左列是下一次的 M.dd（设计稿 .when，下面一行相对日子见 MemoryDetailText.relativeDay）；
// 没日子的卡照 .mem 写正文 + 「种类 · 你写的」。「记一条」是不在分组里的线框按钮，下面是「待整理」（MaterialsSection）。
// 顶部导航条要当下一层的返回字（1d 显示「‹ 小美」），所以标题设成名字再用空的 principal 盖住不显示。

import SwiftUI

struct ContactDetailView: View {
    let store: MemoryStore

    let contactId: String

    @State private var editing: MemoryCard?

    @State private var adding = false

    @State private var materials = MaterialsStore()

    @State private var memoryReady = true

    /// 这次进来要显示的「待整理快满了」；同一个人只出一次。
    @State private var nudge: String?

    @Environment(\.scenePhase) private var scenePhase

    var body: some View {
        PaperPage(spacing: 12) {
            if let error = store.loadError {
                MemoryFailureBanner(text: error) { Task { await store.reload() } }
                    .padding(.horizontal, 20)
            }
            if let nudge {
                Label(nudge, systemImage: "tray.full")
                    .font(AppFont.subheadline)
                    .foregroundStyle(ColorUsage.materialsNudge.role.color)
                    .padding(.horizontal, 20)
            }
            if let contact = store.contact(contactId) {
                header(contact)
                    .padding(.horizontal, 20)
                    .padding(.top, 2)
                    .padding(.bottom, 4)
                cardSections
                addCardButton
                MaterialsSection(store: materials, contactId: contactId, memoryReady: memoryReady)
                #if DEBUG
                MemoryStressSection(store: store, contactId: contactId)
                #endif
            }
        }
        // 只给下一层当返回字用（1d 写「‹ 小美」），这一层自己不显示标题
        .navigationTitle(store.contact(contactId)?.name ?? "")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .principal) { EmptyView() }
            ToolbarItem(placement: .topBarTrailing) {
                NavigationLink("设置", value: MemoryRoute.settings(contactId))
                    .foregroundStyle(ColorUsage.contactSettingsButton.role.color)
            }
        }
        .task { await reloadMaterials() }
        .onChange(of: scenePhase) { _, phase in
            if phase == .active { Task { await reloadMaterials() } }
        }
        .alert(
            materials.message ?? "",
            isPresented: Binding(get: { materials.message != nil }, set: { if !$0 { materials.message = nil } })
        ) {
            Button("好", role: .cancel) {}
        }
        .sheet(item: $editing) { card in CardEditor(store: store, contactId: contactId, card: card) }
        .sheet(isPresented: $adding) { CardEditor(store: store, contactId: contactId, card: nil) }
    }

    /// 重读待整理与开没开素笺云（键盘随时可能再记一笔，开通在别的页面）；读到快满就看要不要提示。
    private func reloadMaterials() async {
        memoryReady = CloudStatus.memoryReady()
        await materials.reload(contactId)
        if nudge == nil, materials.list != nil {
            nudge = MaterialNudge().take(contactId: contactId, count: materials.count)
        }
    }

    // MARK: - 分节

    private var cards: [MemoryCard] { store.cards(of: contactId) }

    /// 日子与约定合成一节：设计稿只有一个「日子与约定」，两类的左列都是那一天，所以按日期排在一起。
    private var datedCards: [MemoryCard] {
        cards.filter(\.kind.hasDate)
            .sorted { ($0.nextDate() ?? .distantFuture, $0.text) < ($1.nextDate() ?? .distantFuture, $1.text) }
    }

    @ViewBuilder
    private var cardSections: some View {
        if !datedCards.isEmpty {
            PaperSectionTitle(text: "日子与约定")
            PaperGroup {
                ForEach(Array(datedCards.enumerated()), id: \.element.id) { index, card in
                    if index > 0 { PaperRowLine() }
                    cardButton(card) { datedRow(card) }
                }
            }
        }
        ForEach(MemoryCard.Kind.allCases.filter { !$0.hasDate }, id: \.self) { kind in
            let list = cards.filter { $0.kind == kind }
            if !list.isEmpty {
                PaperSectionTitle(text: kind.title)
                PaperGroup {
                    ForEach(Array(list.enumerated()), id: \.element.id) { index, card in
                        if index > 0 { PaperRowLine() }
                        cardButton(card) { memRow(card) }
                    }
                }
            }
        }
        if cards.isEmpty {
            PaperGroup {
                PaperRow(title: "还没有写下关于\(store.contact(contactId)?.name ?? "")的事", titleColor: Theme.ink3)
            }
        }
    }

    private var addCardButton: some View {
        HStack {
            PaperLineButton(title: "+ 记一条", height: 36, color: ColorUsage.addCardButton.role.color) {
                adding = true
            }
            .disabled(!store.canEdit)
            .opacity(store.canEdit ? 1 : 0.4)
            Spacer(minLength: 0)
        }
        .padding(.horizontal, 16)
    }

    private func cardButton<Content: View>(_ card: MemoryCard, @ViewBuilder content: () -> Content) -> some View {
        Button {
            editing = card
        } label: {
            content()
        }
        .buttonStyle(.plain)
        .foregroundStyle(Theme.ink)
    }

    private func header(_ contact: MemoryContact) -> some View {
        HStack(spacing: 14) {
            MemoryAvatar(name: contact.name, size: 56, font: AppFont.font(size: 56 * 0.4, weight: .medium))
            VStack(alignment: .leading, spacing: 4) {
                Text(contact.name).font(AppFont.font(size: 26, weight: .semibold))
                Text(MemoryDetailText.subtitle(knownDays: contact.knownDays(), cardCount: cards.count))
                    .font(AppFont.font(size: 12.5))
                    .foregroundStyle(Theme.ink3)
            }
        }
    }

    /// 设计稿 .when：左列 48pt 宽的 M.dd（等宽数字、灰绿）与下面一行相对日子。
    private func datedRow(_ card: MemoryCard) -> some View {
        HStack(spacing: 12) {
            VStack(spacing: 1) {
                Text(card.monthDay() ?? "")
                    .font(AppFont.font(size: 13, weight: .semibold))
                    .foregroundStyle(Theme.accentInk.color)
                    .monospacedDigit()
                if let days = card.daysAway(), let target = card.nextDate(),
                   let label = MemoryDetailText.relativeDay(days: days, target: target) {
                    Text(label).font(AppFont.font(size: 10.5)).foregroundStyle(Theme.ink3)
                }
            }
            .frame(width: 48)
            VStack(alignment: .leading, spacing: 2) {
                Text(card.text).font(AppFont.font(size: 15))
                if !card.subtitle.isEmpty {
                    Text(card.subtitle).font(AppFont.font(size: 12.5)).foregroundStyle(Theme.ink3)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 12)
        .contentShape(Rectangle())
    }

    /// 没日子的卡：正文 + 「喜好 · 你写的」那行。已有一条卡时不重复写种类（小节标题就是它）。
    private func memRow(_ card: MemoryCard) -> some View {
        PaperMemRow(text: card.text, meta: MemoryDetailText.meta(kind: card.kind, source: card.source))
    }
}
