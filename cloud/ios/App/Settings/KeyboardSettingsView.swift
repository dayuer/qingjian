// 键盘设置：拼音方案、模糊音、繁体、全角标点、学习、领域词库、自定义短语，最后是试打框。
// 与 Mac 偏好设置里的同名项写同一个 config.toml，开了素笺云 同步时两边互通。

import SwiftUI

struct KeyboardSettingsView: View {
    let store: SettingsStore

    @State private var draft = ""

    var body: some View {
        Form {
            if let settings = store.settings {
                sections(settings)
            } else {
                Text("设置读不出来").foregroundStyle(.secondary)
            }
        }
        .navigationTitle("键盘设置")
        .onAppear { store.reload() }
        .alert("没有保存", isPresented: errorShown) {
            Button("好") { store.error = nil }
        } message: {
            Text(store.error ?? "")
        }
    }

    @ViewBuilder
    private func sections(_ settings: KeyboardSettings) -> some View {
        Section {
            Picker("拼音方案", selection: binding(\.scheme)) {
                ForEach(settings.schemes, id: \.key) { Text($0.label).tag($0.key) }
            }
        } footer: {
            Text("全拼或双拼；注音在 iPhone 上按全拼。")
        }
        Section("模糊音") {
            ForEach(FuzzyOptions.rows, id: \.label) { row in
                Toggle(row.label, isOn: binding(Self.fuzzy.appending(path: row.path)))
                    .tint(ColorUsage.appToggle.role.color)
            }
        }
        Section {
            Toggle("繁体输出", isOn: binding(\.traditional))
                .tint(ColorUsage.appToggle.role.color)
            Toggle("中文标点用全角", isOn: binding(\.fullWidthPunctuation))
                .tint(ColorUsage.appToggle.role.color)
            Toggle("学习输入习惯", isOn: binding(\.learning))
                .tint(ColorUsage.appToggle.role.color)
            Toggle("云联想", isOn: binding(\.cloudPrediction))
                .tint(ColorUsage.appToggle.role.color)
        } footer: {
            Text("关掉学习后不再记新词与词频，已学到的保留。云联想经素笺云 让大模型补候选与整句，和 Mac 是同一个开关。")
        }
        if !settings.domains.isEmpty {
            Section("领域词库") {
                ForEach(settings.domains.indices, id: \.self) { index in
                    Toggle(settings.domains[index].label, isOn: binding(\.domains[index].enabled))
                        .tint(ColorUsage.appToggle.role.color)
                }
            }
        }
        Section {
            NavigationLink {
                PhrasesView(store: store)
            } label: {
                LabeledContent("自定义短语", value: "\(settings.phrases.count)")
            }
        }
        Section("试一试") {
            TextField("在这里试打", text: $draft, axis: .vertical)
                .lineLimit(3...8)
        }
    }

    private static let fuzzy: WritableKeyPath<KeyboardSettings, FuzzyOptions> = \.fuzzy

    private func binding<T>(_ path: WritableKeyPath<KeyboardSettings, T>) -> Binding<T> {
        Binding(
            get: { store.settings![keyPath: path] },
            set: { value in store.error = store.update { $0[keyPath: path] = value } })
    }

    private var errorShown: Binding<Bool> {
        Binding(get: { store.error != nil }, set: { if !$0 { store.error = nil } })
    }
}
