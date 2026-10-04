// 对象详情（02 的 1b，2A 没有「待确认」）：56pt 头像字、衬线名字、「认识 n 天 · n 条记忆」，卡片按日子 / 约定 / 喜好 / 近况 / 其他分组；右上「设置」。
// 有日子的卡左列是下一次的 M.dd（设计稿 .when：衬线 13pt 灰绿），下面一行相对日子（MemoryDetailText.relativeDay）；
// 没日子的卡照 .mem 行：15pt 正文，下面 11.5pt 灰字「种类 · 你写的」。「记一条」按钮设计稿 iOS 版没画，保留。

import SwiftUI

struct ContactDetailView: View {
    let store: MemoryStore

    let contactId: String

    @State private var editing: MemoryCard?

    @State private var adding = false

    var body: some View {
        List {
            if let error = store.loadError {
                MemoryFailureBanner(text: error) { Task { await store.reload() } }
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
                #if DEBUG
                MemoryStressSection(store: store, contactId: contactId)
                #endif
            }
        }
        .navigationTitle("")
        .navigationBarTitleDisplayMode(.inline)
        .toolbar {
            ToolbarItem(placement: .topBarTrailing) {
                NavigationLink("设置", value: MemoryRoute.settings(contactId))
                    .foregroundStyle(ColorUsage.contactSettingsButton.role.color)
            }
        }
        .sheet(item: $editing) { card in CardEditor(store: store, contactId: contactId, card: card) }
        .sheet(isPresented: $adding) { CardEditor(store: store, contactId: contactId, card: nil) }
    }

    private func header(_ contact: MemoryContact) -> some View {
        HStack(spacing: 14) {
            MemoryAvatar(name: contact.name, size: 56, scene: contact.scene, serif: true)
            VStack(alignment: .leading, spacing: 4) {
                Text(contact.name).font(.system(size: 26, weight: .semibold, design: .serif))
                Text(MemoryDetailText.subtitle(knownDays: contact.knownDays(), cardCount: store.cards(of: contactId).count))
                    .font(.system(size: 12.5))
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
                        .font(.system(size: 13, weight: .semibold, design: .serif))
                        .foregroundStyle(Theme.accentInk.color)
                        .monospacedDigit()
                    if let days = card.daysAway(), let target = card.nextDate(),
                       let label = MemoryDetailText.relativeDay(days: days, target: target) {
                        Text(label).font(.system(size: 10.5)).foregroundStyle(Theme.ink3)
                    }
                }
                .frame(width: 48)
                VStack(alignment: .leading, spacing: 2) {
                    Text(card.text).font(.system(size: 15))
                    if !card.subtitle.isEmpty {
                        Text(card.subtitle).font(.system(size: 12.5)).foregroundStyle(Theme.ink3)
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
            }
            .contentShape(Rectangle())
        } else {
            VStack(alignment: .leading, spacing: 4) {
                Text(card.text).font(.system(size: 15)).lineSpacing(3)
                Text(MemoryDetailText.meta(kind: card.kind, source: card.source))
                    .font(.system(size: 11.5))
                    .foregroundStyle(Theme.ink3)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .contentShape(Rectangle())
        }
    }
}
