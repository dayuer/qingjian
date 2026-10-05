// 「键盘记住的事」首页（05 的 2i），挂在「记得」Tab：顶上「本周」入口（T5 并进首页前的过渡）、副标题、3 天内的提醒卡（恋爱与日常的人）、
// 按场景分组的人（恋爱、日常、工作，各自 n / 8，每组下面「加一个人」，从哪组点进去就建在哪个场景；工作组的头像用中性色）、底部灰底「懒得自己写？」。
// 读不出来时顶上常驻原因（MemoryFailureBanner），不让列表静默地空着。

import SwiftUI

struct MemoryHomeView: View {
    let store: MemoryStore

    /// 正在哪个场景里「加一个人」；nil 时弹层收着。
    @State private var addingScene: String?

    @State private var path: [MemoryRoute] = []

    var body: some View {
        NavigationStack(path: $path) {
            List {
                Section {
                    NavigationLink("本周") { WeekView(store: store) }
                }
                Section {
                    if let error = store.loadError {
                        MemoryFailureBanner(text: error) { Task { await store.reload() } }
                    } else if !store.loaded {
                        HStack(spacing: 8) {
                            ProgressView()
                            Text(MemoryStore.Wording.loading).foregroundStyle(.secondary)
                        }
                    }
                    ForEach(store.upcoming(within: 3)) { item in
                        NavigationLink(value: MemoryRoute.contact(item.contact.id)) { reminderRow(item) }
                            .listRowBackground(ColorUsage.reminderCard.role.color)
                    }
                } header: {
                    Text("都是你写的 · 只存在这台手机上").textCase(nil)
                }
                ForEach(store.groups) { group in
                    Section {
                        if store.canEdit && store.snapshot.contacts.isEmpty && group.scene == MemoryScope.homeOrder.first {
                            Text("还没有记下任何人。加一个人，打字时键盘就能想起 TA 的事。")
                                .font(AppFont.subheadline)
                                .foregroundStyle(.secondary)
                        }
                        ForEach(group.people) { contact in
                            NavigationLink(value: MemoryRoute.contact(contact.id)) { personRow(contact) }
                        }
                        if !group.isFull {
                            Button {
                                addingScene = group.scene
                            } label: {
                                addRow
                            }
                            .disabled(!store.canEdit)
                            .opacity(store.canEdit ? 1 : 0.4)
                        }
                    } header: {
                        Text(group.header)
                    } footer: {
                        if group.isFull { Text(group.fullNote) }
                    }
                }
                Section {
                    NavigationLink {
                        CloudIntroView()
                    } label: {
                        VStack(alignment: .leading, spacing: 4) {
                            Text("懒得自己写？").font(AppFont.subheadline.weight(.medium))
                            Text("以后开通素笺云服务，键盘会在你选的场景里自己记、每天整理，你只用每周确认一下。")
                                .font(AppFont.caption)
                                .foregroundStyle(.secondary)
                            Text("了解一下")
                                .font(AppFont.caption.weight(.medium))
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
            .sheet(isPresented: adding) { ContactEditor(store: store, scene: addingScene ?? MemoryScope.dating) }
            .refreshable { await store.reload() }
        }
    }

    private func reminderRow(_ item: MemoryUpcoming) -> some View {
        HStack(spacing: 14) {
            Text(item.shortDay)
                .font(AppFont.subheadline.weight(.semibold))
                .foregroundStyle(ColorUsage.reminderDay.role.color)
                .frame(minWidth: 40, alignment: .leading)
            Text(item.title)
        }
    }

    private func personRow(_ contact: MemoryContact) -> some View {
        HStack(spacing: 12) {
            MemoryAvatar(name: contact.name, size: 36, scene: contact.scene, font: AppFont.font(size: 36 * 0.45, weight: .semibold))
            VStack(alignment: .leading, spacing: 2) {
                Text(contact.name)
                Text("\(store.cards(of: contact.id).count) 件")
                    .font(AppFont.caption)
                    .foregroundStyle(.secondary)
            }
        }
    }

    private var adding: Binding<Bool> {
        Binding(get: { addingScene != nil }, set: { if !$0 { addingScene = nil } })
    }

    private var addRow: some View {
        HStack(spacing: 12) {
            Image(systemName: "plus")
                .font(AppFont.font(size: 15, weight: .medium))
                .frame(width: 36, height: 36)
                .overlay(Circle().strokeBorder(Color(.separator)))
            Text("加一个人")
        }
        .foregroundStyle(ColorUsage.addContactButton.role.color)
    }
}
