// 「+ 记一条」的输入层：**只写一句话，不问是谁、不选种类**。
// 记下就走 MemoryStore.quickNote 落成「还没归到人的」卡，整理留到事后在首页用「补上」归人。

import SwiftUI

struct QuickNoteSheet: View {
    let store: MemoryStore

    @Environment(\.dismiss) private var dismiss

    @State private var text = ""

    @FocusState private var focused: Bool

    private var trimmed: String {
        text.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    var body: some View {
        NavigationStack {
            VStack(alignment: .leading, spacing: 12) {
                TextEditor(text: $text)
                    .font(AppFont.font(size: 16))
                    .frame(minHeight: 120)
                    .padding(8)
                    .background(
                        RoundedRectangle(cornerRadius: 12)
                            .fill(Color(.secondarySystemGroupedBackground)))
                    .overlay(
                        RoundedRectangle(cornerRadius: 12).strokeBorder(Color(.separator)))
                    .focused($focused)
                Text("先记下就好，之后在首页用「补上」归给某个人。")
                    .font(AppFont.footnote)
                    .foregroundStyle(.secondary)
                Spacer()
            }
            .padding(16)
            .navigationTitle("记一条")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("取消") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("记下") {
                        Task {
                            if await store.quickNote(trimmed) { dismiss() }
                        }
                    }
                    .disabled(trimmed.isEmpty || store.saving)
                }
            }
            .onAppear { focused = true }
        }
    }
}
