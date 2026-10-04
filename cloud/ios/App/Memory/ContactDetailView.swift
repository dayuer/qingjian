// 对象详情（02 的 1b，2A 没有「待确认」）：头像字、名字、认识几天，卡片按日子 / 约定 / 喜好 / 近况 / 其他分组；右上「设置」。
// 日子与约定左列是下一次的 M.dd，3 天内底下再写相对的日子。

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
                    HStack(spacing: 14) {
                        MemoryAvatar(name: contact.name, size: 56, scene: contact.scene)
                        VStack(alignment: .leading, spacing: 4) {
                            Text(contact.name).font(.system(.title2, design: .serif).weight(.semibold))
                            Text("认识 \(contact.knownDays()) 天 · \(store.cards(of: contactId).count) 件")
                                .font(.subheadline)
                                .foregroundStyle(.secondary)
                        }
                    }
                    .listRowBackground(Color.clear)
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
                                .foregroundStyle(.primary)
                            }
                        }
                    }
                }
                if store.cards(of: contactId).isEmpty {
                    Section {
                        Text("还没有写下关于\(contact.name)的事").foregroundStyle(.secondary)
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

    private func row(_ card: MemoryCard) -> some View {
        HStack(alignment: .firstTextBaseline, spacing: 14) {
            if let monthDay = card.monthDay() {
                VStack(alignment: .leading, spacing: 2) {
                    Text(monthDay).font(.subheadline.weight(.semibold)).monospacedDigit()
                    if let days = card.daysAway(), (0...3).contains(days), let label = card.dateLabel() {
                        Text(label).font(.caption2).foregroundStyle(.secondary)
                    }
                }
                .frame(minWidth: 44, alignment: .leading)
            }
            VStack(alignment: .leading, spacing: 2) {
                Text(card.text)
                if !card.subtitle.isEmpty {
                    Text(card.subtitle).font(.caption).foregroundStyle(.secondary)
                }
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .contentShape(Rectangle())
    }
}
