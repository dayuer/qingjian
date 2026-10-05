// 「通讯录」Tab（设计稿 02 的 2b）：衬线大标题与右上「+」、搜索框、按首字母分的通栏列表与右侧字母索引。
// 行是 17pt 名字 + 13pt 灰字场景 + 行尾灰绿事件提示；点一行在下面展开这个人的几张记忆卡（见 `ContactRow`）。
//
// 场景是用户自建的分组（「我」页里增删改名），所以「+」建人时要先问建在哪个场景。

import SwiftUI

struct ContactsView: View {
    let store: MemoryStore

    @State private var path: [MemoryRoute] = []

    @State private var search = ""

    /// 展开了记忆卡的那个人；同时只展开一个。
    @State private var expanded: String?

    /// 「+」选完场景后建在哪个场景（默认第一个）。
    @State private var newScene = ""

    /// 弹建人页。
    @State private var adding = false

    var body: some View {
        NavigationStack(path: $path) {
            VStack(spacing: 0) {
                PageHeader(title: "通讯录") { addButton }
                searchField
                list
            }
            .background(Color(.systemBackground))
            .toolbar(.hidden, for: .navigationBar)
            .navigationDestination(for: MemoryRoute.self) { route in
                switch route {
                case .contact(let id): ContactDetailView(store: store, contactId: id)
                case .settings(let id): ContactSettingsView(store: store, contactId: id)
                }
            }
            // 忘掉了某个人（设置页里，或键盘那边改了名单后重读）：退掉他的页面，收起他的展开区
            .onChange(of: store.snapshot.contacts.map(\.id)) { _, ids in
                path.removeAll { !ids.contains($0.contactId) }
                if let id = expanded, !ids.contains(id) { expanded = nil }
            }
            .sheet(isPresented: $adding) {
                ContactEditor(store: store, scene: newScene.isEmpty ? (store.defaultScene?.id ?? "") : newScene)
            }
        }
    }

    // MARK: - 顶部

    /// 「+」：先选建在哪个场景，再开建人页。
    private var addButton: some View {
        Menu {
            ForEach(store.scenes) { scene in
                Button(scene.name) {
                    newScene = scene.id
                    adding = true
                }
            }
        } label: {
            Image(systemName: "plus")
                .font(AppFont.font(size: 22))
                .foregroundStyle(Theme.ink)
                .frame(width: 28, height: 28)
        }
        .disabled(!store.canEdit)
        .opacity(store.canEdit ? 1 : 0.4)
        .accessibilityLabel("加一个人")
        .accessibilityIdentifier("addContact")
    }

    /// 搜索框（设计稿 2b）：36pt 高、10 圆角、浅灰底，占位「搜索 n 个人」。
    private var searchField: some View {
        HStack(spacing: 6) {
            TextField(
                "", text: $search,
                prompt: Text(ContactIndex.searchPrompt(count: store.snapshot.contacts.count))
                    .foregroundStyle(Theme.ink3)
            )
            .font(AppFont.font(size: 14))
            .textInputAutocapitalization(.never)
            .autocorrectionDisabled()
            .submitLabel(.search)
            if !search.isEmpty {
                Button {
                    search = ""
                } label: {
                    Image(systemName: "xmark.circle.fill")
                        .font(AppFont.font(size: 15))
                        .foregroundStyle(Theme.ink3)
                }
                .buttonStyle(.plain)
                .accessibilityLabel("清空搜索")
            }
        }
        .padding(.horizontal, 12)
        .frame(height: 36)
        .background(Color(.secondarySystemBackground), in: RoundedRectangle(cornerRadius: 10))
        .padding(.horizontal, 16)
        .padding(.top, 6)
    }

    // MARK: - 列表

    private var list: some View {
        ScrollViewReader { proxy in
            ScrollView {
                LazyVStack(spacing: 0) {
                    if let error = store.loadError {
                        MemoryFailureBanner(text: error) { Task { await store.reload() } }
                            .padding(.horizontal, 16)
                    }
                    if visible.isEmpty {
                        emptyState
                    } else {
                        ForEach(sections) { section in
                            if !searching {
                                letterHeader(section.letter)
                                    .id(section.letter)
                            }
                            ForEach(section.people) { contact in
                                ContactRow(
                                    contact: contact,
                                    sceneName: store.sceneName(of: contact.scene),
                                    note: notes[contact.id],
                                    cards: store.cards(of: contact.id),
                                    expanded: expanded == contact.id,
                                    onToggle: { toggle(contact.id) },
                                    onOpenDetail: { path.append(.contact(contact.id)) })
                            }
                        }
                    }
                }
                .padding(.top, 8)
                .padding(.bottom, 24)
            }
            .scrollDismissesKeyboard(.immediately)
            .overlay(alignment: .topTrailing) {
                // 搜索时不分组，也就没有索引；只有一个字母也不值得摆一条
                if !searching, letters.count > 1 {
                    LetterIndex(letters: letters) { letter in
                        withAnimation { proxy.scrollTo(letter, anchor: .top) }
                    }
                    .padding(.top, 12)
                    .padding(.trailing, 3)
                }
            }
        }
    }

    private func letterHeader(_ letter: String) -> some View {
        Text(letter)
            .font(AppFont.font(size: 13, weight: .semibold))
            .foregroundStyle(Theme.ink3)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.top, 4)
            .frame(height: 24)
            .overlay(alignment: .bottom) {
                Rectangle().fill(Color(.separator).opacity(0.6)).frame(height: 0.5)
            }
            .padding(.leading, 20)
            .padding(.trailing, 24)
    }

    private var emptyState: some View {
        Text(searching ? "没有找到「\(search)」" : "还没有记着的人")
            .font(AppFont.font(size: 14))
            .foregroundStyle(Theme.ink3)
            .frame(maxWidth: .infinity)
            .padding(.top, 80)
    }

    // MARK: - 数据

    private var searching: Bool {
        !search.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
    }

    private var visible: [MemoryContact] {
        ContactIndex.search(store.snapshot.contacts, text: search) { store.sceneName(of: $0) }
    }

    private var sections: [ContactSection] { ContactIndex.sections(visible) }

    private var letters: [String] { ContactIndex.letters(sections) }

    /// 行尾的事件提示：与首页 7 天日历条同一个跨度（6 天内）。
    private var notes: [String: String] { ContactIndex.notes(store.upcoming(within: 6)) }

    private func toggle(_ id: String) {
        withAnimation(.snappy(duration: 0.18)) {
            expanded = expanded == id ? nil : id
        }
    }
}
