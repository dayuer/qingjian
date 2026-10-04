// 编辑一条自定义短语；保存时由桥校验（输入码只能是小写字母、位置 1–9 等），不合法就留在这里显示原因。

import SwiftUI

struct PhraseEditor: View {
    @State var phrase: Phrase

    /// 写回设置，成功返回 nil，失败返回原因。
    let save: (Phrase) -> String?

    @State private var error: String?

    @Environment(\.dismiss) private var dismiss

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    TextField("输入码，如 dz", text: $phrase.code)
                        .textInputAutocapitalization(.never)
                        .autocorrectionDisabled()
                    TextField("上屏的文字", text: $phrase.text, axis: .vertical)
                        .lineLimit(1...6)
                    Stepper("第 \(phrase.position) 个候选", value: $phrase.position, in: 1...9)
                    Toggle("启用", isOn: $phrase.enabled)
                        .tint(ColorUsage.appToggle.role.color)
                } footer: {
                    if let error { Text(error).foregroundStyle(.red) }
                }
            }
            .navigationTitle("自定义短语")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("取消") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("保存") {
                        error = save(phrase)
                        if error == nil { dismiss() }
                    }
                }
            }
        }
    }
}
