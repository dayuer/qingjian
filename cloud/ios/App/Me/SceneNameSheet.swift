// 场景名输入弹层：新建与改名共用（一个文本框 + 「存好」）。
// 名字的长度与空值在这里先挡一道，桥那边还会再校验一次。

import SwiftUI

struct SceneNameSheet: View {
    let title: String

    /// 打开时填进去的名字；新建时为空。
    let name: String

    let onSave: (String) -> Void

    @Environment(\.dismiss) private var dismiss

    @State private var text = ""

    @FocusState private var focused: Bool

    var body: some View {
        NavigationStack {
            Form {
                TextField("场景名", text: $text)
                    .focused($focused)
                    .onChange(of: text) { _, value in
                        let clamped = String(value.prefix(MemoryScene.maxNameChars))
                        if clamped != value { text = clamped }
                    }
            }
            .navigationTitle(title)
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("取消") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("存好") {
                        onSave(text)
                        dismiss()
                    }
                    .disabled(trimmed.isEmpty)
                }
            }
            .onAppear {
                text = name
                focused = true
            }
        }
    }

    private var trimmed: String {
        text.trimmingCharacters(in: .whitespacesAndNewlines)
    }
}
