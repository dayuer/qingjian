// 对象详情（02 的 1b）卡片分组下面的「待整理 · n 条」：记一笔存下的原话，新的在上，等素笺云每天整理成记忆卡（设计稿没画这一节，UI 清单约束 7 第 17 条）。
// 每条可以展开看全文、删除（删前确认一次）；读不出来时这一节显示原因与「再读一次」。
// 没开素笺云时组下面一行「开通素笺云后，每天帮你整理成记忆卡」，点了进 CloudIntroView。一条都没有时整节不出。

import SwiftUI

struct MaterialsSection: View {
    let store: MaterialsStore

    let contactId: String

    /// 素材会被整理（CloudStatus.memoryReady：开通并同意了「记忆」）：会就不放开通引导。
    let memoryReady: Bool

    @State private var expanded: Set<String> = []

    @State private var confirming: MemoryMaterial?

    var body: some View {
        Group {
            if let error = store.loadError {
                PaperSectionTitle(text: MaterialDisplay.title(count: store.count))
                PaperGroup {
                    MemoryFailureBanner(text: error) { Task { await store.reload(contactId) } }
                        .padding(.horizontal, 14)
                        .padding(.vertical, 12)
                }
            } else if let list = store.list, list.unprocessedCount > 0 {
                PaperSectionTitle(text: MaterialDisplay.title(count: list.unprocessedCount))
                PaperGroup {
                    ForEach(Array(list.materials.enumerated()), id: \.element.id) { index, material in
                        if index > 0 { PaperRowLine() }
                        MaterialRow(
                            material: material,
                            expanded: expanded.contains(material.id),
                            deleting: store.deleting == material.id,
                            toggle: { toggle(material.id) },
                            delete: { confirming = material })
                    }
                }
                if !memoryReady {
                    NavigationLink {
                        CloudIntroView()
                    } label: {
                        Text(MaterialDisplay.cloudHint)
                            .font(AppFont.font(size: 12))
                            .foregroundStyle(Theme.ink3)
                            .padding(.horizontal, 20)
                            .frame(maxWidth: .infinity, alignment: .leading)
                    }
                    .buttonStyle(.plain)
                }
            }
        }
        // 用 .alert 不用 confirmationDialog：iOS 26 上后者是气泡，「取消」不显示（同「忘掉」）
        .alert(
            MaterialDisplay.deleteTitle,
            isPresented: Binding(get: { confirming != nil }, set: { if !$0 { confirming = nil } }),
            presenting: confirming
        ) { material in
            Button(MaterialDisplay.deleteButton, role: .destructive) {
                Task { await store.delete(material, contactId: contactId) }
            }
            Button("取消", role: .cancel) {}
        } message: { _ in
            Text(MaterialDisplay.deleteMessage)
        }
    }

    private func toggle(_ id: String) {
        if expanded.contains(id) {
            expanded.remove(id)
        } else {
            expanded.insert(id)
        }
    }
}
