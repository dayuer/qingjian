// 青简 Cloud：显示连的哪台服务器（随安装配好，不能改），各项功能开关可调；键盘下次弹出时按新开关重连。

import SwiftUI

struct CloudSettingsView: View {
    let store: SettingsStore

    @State private var message: String?

    var body: some View {
        Form {
            if let cloud = store.cloud {
                Section {
                    LabeledContent("服务器", value: cloud.server.isEmpty ? "未配置" : cloud.server)
                    LabeledContent("状态", value: cloud.connected ? "已配置" : "未配置，键盘离线使用")
                }
                Section {
                    Toggle("同步学习数据与设置", isOn: field(\.sync))
                    Toggle("上传输入日志（服务器据此纠错调频）", isOn: field(\.logs))
                    Toggle("跨设备剪贴板", isOn: field(\.clipboard))
                    Toggle("大模型润色", isOn: field(\.llm))
                    Toggle("组字时大模型补候选", isOn: field(\.candidates))
                        .disabled(!(store.cloud?.llm ?? false))
                } footer: {
                    Text("这些功能都需要在系统设置里给青简打开「允许完全访问」。")
                }
                .disabled(!cloud.connected)
                Section {
                    Button("保存") {
                        message = store.saveCloud() ?? "已保存，键盘下次弹出时生效"
                    }
                } footer: {
                    if let message { Text(message) }
                }
            }
        }
        .navigationTitle("青简 Cloud")
    }

    private func field<T>(_ path: WritableKeyPath<CloudSettings, T>) -> Binding<T> {
        Binding(
            get: { store.cloud![keyPath: path] },
            set: { store.cloud![keyPath: path] = $0 })
    }
}
