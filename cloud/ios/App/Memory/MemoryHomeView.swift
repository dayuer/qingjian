// 「键盘记住的事」首页（05 的 2i）：副标题、3 天内的提醒卡、恋爱场景的人（n / 8）、加一个人、底部灰底「懒得自己写？」。
// 读不出来时顶上常驻原因（MemoryFailureBanner），不让列表静默地空着。

import SwiftUI

struct MemoryHomeView: View {
    let store: MemoryStore

    @State private var addingContact = false

    @State private var path: [MemoryRoute] = []

    var body: some View {
        NavigationStack(path: $path) {
            List {
                Section {
                    if let error = store.loadError {
                        MemoryFailureBanner(text: error) { store.reload() }
                    }
                    ForEach(store.upcoming(within: 3)) { item in
                        NavigationLink(value: MemoryRoute.contact(item.contact.id)) { reminderRow(item) }
                            .listRowBackground(Theme.accentSoft.color)
                    }
                } header: {
                    Text("都是你写的 · 只存在这台手机上").textCase(nil)
                }
                Section {
                    if store.canEdit && store.people.isEmpty {
                        Text("还没有记下任何人。加一个人，打字时键盘就能想起 TA 的事。")
                            .font(.subheadline)
                            .foregroundStyle(.secondary)
                    }
                    ForEach(store.people) { contact in
                        NavigationLink(value: MemoryRoute.contact(contact.id)) { personRow(contact) }
                    }
                    if store.people.count < MemoryStore.contactLimit {
                        Button {
                            addingContact = true
                        } label: {
                            addRow
                        }
                        .disabled(!store.canEdit)
                        .opacity(store.canEdit ? 1 : 0.4)
                    }
                } header: {
                    Text("人 · \(store.people.count) / \(MemoryStore.contactLimit)")
                } footer: {
                    if store.people.count >= MemoryStore.contactLimit { Text("恋爱场景最多 8 个人") }
                }
                Section {
                    NavigationLink {
                        CloudIntroView()
                    } label: {
                        VStack(alignment: .leading, spacing: 4) {
                            Text("懒得自己写？").font(.subheadline.weight(.medium))
                            Text("以后开通素笺云服务，键盘会在你选的场景里自己记、每天整理，你只用每周确认一下。")
                                .font(.caption)
                                .foregroundStyle(.secondary)
                            Text("了解一下")
                                .font(.caption.weight(.medium))
                                .foregroundStyle(ColorUsage.cloudIntroLink.role.color)
                        }
                        .padding(.vertical, 2)
                    }
                    .listRowBackground(Color(.secondarySystemFill))
                }
            }
            .navigationTitle("键盘记住的事")
            .navigationDestination(for: MemoryRoute.self) { route in
                switch route {
                case .contact(let id): ContactDetailView(store: store, contactId: id)
                case .settings(let id): ContactSettingsView(store: store, contactId: id)
                }
            }
            // 忘掉了某个人（设置页里，或键盘那边改了名单后重读）：退掉这个人的详情与设置页
            .onChange(of: store.snapshot.contacts.map(\.id)) { _, ids in
                path.removeAll { !ids.contains($0.contactId) }
            }
            .sheet(isPresented: $addingContact) { ContactEditor(store: store) }
            .task { store.reload() }
            .refreshable { store.reload() }
        }
    }

    private func reminderRow(_ item: MemoryUpcoming) -> some View {
        HStack(spacing: 14) {
            Text(item.shortDay)
                .font(.subheadline.weight(.semibold))
                .foregroundStyle(Theme.accentInk.color)
                .frame(minWidth: 40, alignment: .leading)
            Text(item.title)
        }
    }

    private func personRow(_ contact: MemoryContact) -> some View {
        HStack(spacing: 12) {
            MemoryAvatar(name: contact.name, size: 36)
            VStack(alignment: .leading, spacing: 2) {
                Text(contact.name)
                Text("\(store.cards(of: contact.id).count) 件")
                    .font(.caption)
                    .foregroundStyle(.secondary)
            }
        }
    }

    private var addRow: some View {
        HStack(spacing: 12) {
            Image(systemName: "plus")
                .font(.system(size: 15, weight: .medium))
                .frame(width: 36, height: 36)
                .overlay(Circle().strokeBorder(Color(.separator)))
            Text("加一个人")
        }
        .foregroundStyle(ColorUsage.addContactButton.role.color)
    }
}
