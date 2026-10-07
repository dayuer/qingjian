// 对象详情（02 的 1b，2A 没有「待确认」）：56pt 头像字、名字（设计稿是衬线，App 统一用 MiSans）、「认识 n 天 · n 条记忆」，卡片按日子 / 约定 / 喜好 / 近况 / 其他分组；右上「设置」。
// 有日子的卡左列是下一次的 M.dd（设计稿 .when：衬线 13pt 灰绿），下面一行相对日子（MemoryDetailText.relativeDay）；
// 没日子的卡照 .mem 行：15pt 正文，下面 11.5pt 灰字「种类 · 你写的」。「记一条」按钮设计稿 iOS 版没画，保留。
// 卡片与「记一条」下面是「待整理」（MaterialsSection，记一笔存下的原话）；快满 180 条时顶上提示一次（MaterialNudge）。

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
        List {
            if let error = store.loadError {
                MemoryFailureBanner(text: error) { Task { await store.reload() } }
            }
            if let nudge {
                Label(nudge, systemImage: "tray.full")
                    .font(AppFont.subheadline)
                    .foregroundStyle(ColorUsage.materialsNudge.role.color)
                    .padding(.vertical, 4)
            }
            if let contact = store.contact(contactId) {
                Section {
                    header(contact)
                        .listRowBackground(Color.clear)
                        .listRowInsets(EdgeInsets(top: 2, leading: 4, bottom: 4, trailing: 4))
                }
                ForEach(MemoryCard.Kind.allCases, id: \.self) { kind in
                    let cards = store.cards(of: contactId).filter { $0.kind == kind }
                    if !cards.isEmpty {
                        Section(kind.title) {
                            ForEach(cards) { card in
                                Button {
                                    editing = card
                                } label: {
                                    row(card)
                                }
                                .foregroundStyle(Theme.ink)
                            }
                        }
                    }
                }
                if store.cards(of: contactId).isEmpty {
                    Section {
                        Text("还没有写下关于\(contact.name)的事").foregroundStyle(Theme.ink3)
                    }
                }
                Section {
                    Button {
                        adding = true
                    } label: {
                        Label("记一条", systemImage: "plus")
                    }
                    .foregroundStyle(ColorUsage.addCardButton.role.color)
                    .disabled(!store.canEdit)
                    .opacity(store.canEdit ? 1 : 0.4)
                }
                MaterialsSection(store: materials, contactId: contactId, memoryReady: memoryReady)
                #if DEBUG
                MemoryStressSection(store: store, contactId: contactId)
                #endif
            }
        }
        .contentMargins(.bottom, RootTab.listBottomMargin, for: .scrollContent)
        .navigationTitle("")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
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

    private func header(_ contact: MemoryContact) -> some View {
        HStack(spacing: 14) {
            MemoryAvatar(name: contact.name, size: 56, font: AppFont.font(size: 56 * 0.4, weight: .medium))
            VStack(alignment: .leading, spacing: 4) {
                Text(contact.name).font(AppFont.font(size: 26, weight: .semibold))
                Text(MemoryDetailText.subtitle(knownDays: contact.knownDays(), cardCount: store.cards(of: contactId).count))
                    .font(AppFont.font(size: 12.5))
                    .foregroundStyle(Theme.ink3)
            }
        }
    }

    @ViewBuilder
    private func row(_ card: MemoryCard) -> some View {
        if let monthDay = card.monthDay() {
            HStack(spacing: 12) {
                VStack(spacing: 1) {
                    Text(monthDay)
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
            .contentShape(Rectangle())
        } else {
            // 设计稿 .mem 只在有话可说时（「覆盖了『喜欢猫』」，2C 才有）写 meta；种类已经是小节标题，不重复
            VStack(alignment: .leading, spacing: 4) {
                Text(card.text).font(AppFont.font(size: 15)).lineSpacing(3)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .contentShape(Rectangle())
        }
    }
}
