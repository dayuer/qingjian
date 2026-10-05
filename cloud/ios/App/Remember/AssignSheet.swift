// 「补上」：把一张「还没归到人的」卡归给某个人。按场景分组列出本机已有的人，点谁就归到谁名下。
// 一个人都没有时给一句提示（先去通讯录加一个人）。

import SwiftUI

struct AssignSheet: View {
    let store: MemoryStore

    let card: MemoryCard

    let onPicked: () -> Void

    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            List {
                if store.snapshot.contacts.isEmpty {
                    Section {
                        Text("还没有记下任何人。先去通讯录加一个人，再回来归这张卡。")
                            .font(AppFont.subheadline)
                            .foregroundStyle(.secondary)
                    }
                }
                ForEach(store.groups) { group in
                    if !group.people.isEmpty {
                        Section(group.title) {
                            ForEach(group.people) { contact in
                                Button {
                                    Task {
                                        if await store.assign(card.id, to: contact.id) {
                                            onPicked()
                                            dismiss()
                                        }
                                    }
                                } label: {
                                    HStack(spacing: 12) {
                                        MemoryAvatar(
                                            name: contact.name, size: 34, scene: contact.scene)
                                        Text(contact.name).foregroundStyle(Theme.ink)
                                        Spacer()
                                    }
                                }
                                .disabled(store.saving)
                            }
                        }
                    }
                }
            }
            .navigationTitle("记到谁那儿？")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("取消") { dismiss() }
                }
            }
        }
        .presentationDetents([.medium, .large])
    }
}
