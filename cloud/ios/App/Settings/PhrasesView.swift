// 自定义短语列表：左滑删除，点一条改，右上角加。

import SwiftUI

struct PhrasesView: View {
    let store: SettingsStore

    /// 正在编辑的短语与它的下标（新加的为 nil）。
    @State private var editing: (index: Int?, phrase: Phrase)?

    var body: some View {
        List {
            ForEach(Array(phrases.enumerated()), id: \.offset) { index, phrase in
                Button {
                    editing = (index, phrase)
                } label: {
                    row(phrase)
                }
                .foregroundStyle(.primary)
            }
            .onDelete { offsets in
                store.update { $0.phrases.remove(atOffsets: offsets) }
            }
        }
        .overlay {
            if phrases.isEmpty {
                ContentUnavailableView(
                    "没有自定义短语", systemImage: "text.badge.plus",
                    description: Text("比如敲 dz 时第 1 个候选固定是你的地址。"))
            }
        }
        .navigationTitle("自定义短语")
        .toolbar {
            Button("添加", systemImage: "plus") {
                editing = (nil, Phrase(code: "", text: "", position: 1, enabled: true))
            }
        }
        .sheet(isPresented: Binding(get: { editing != nil }, set: { if !$0 { editing = nil } })) {
            if let editing {
                PhraseEditor(phrase: editing.phrase) { phrase in
                    store.update { settings in
                        if let index = editing.index {
                            settings.phrases[index] = phrase
                        } else {
                            settings.phrases.append(phrase)
                        }
                    }
                }
            }
        }
    }

    private var phrases: [Phrase] { store.settings?.phrases ?? [] }

    private func row(_ phrase: Phrase) -> some View {
        HStack {
            VStack(alignment: .leading) {
                Text(phrase.text).lineLimit(1)
                Text("\(phrase.code) · 第 \(phrase.position) 位")
                    .font(.caption).foregroundStyle(.secondary)
            }
            Spacer()
            if !phrase.enabled {
                Text("停用").font(.caption).foregroundStyle(.secondary)
            }
        }
    }
}
