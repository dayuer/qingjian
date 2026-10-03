// 启用步骤 + 试打框。

import SwiftUI
import UIKit

struct SetupView: View {
    @State private var draft = ""

    var body: some View {
        NavigationStack {
            Form {
                Section("启用键盘") {
                    Label("打开「设置 → 通用 → 键盘 → 键盘」", systemImage: "1.circle")
                    Label("点「添加新键盘…」，选「青简」", systemImage: "2.circle")
                    Label("打字时长按地球键切到青简", systemImage: "3.circle")
                    Button("打开设置") {
                        if let url = URL(string: UIApplication.openSettingsURLString) {
                            UIApplication.shared.open(url)
                        }
                    }
                }
                Section {
                    TextField("在这里试打", text: $draft, axis: .vertical)
                        .lineLimit(3...8)
                } header: {
                    Text("试一试")
                } footer: {
                    Text("「完全访问」用于按键震动，以及连接你自己的青简 Cloud 服务器（大模型联想、润色、与 Mac 同步学习数据）。不开也能正常打字；没配 Cloud 时键盘不联网。")
                }
            }
            .navigationTitle("青简")
        }
    }
}
