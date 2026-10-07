// 设备列表的一行：名字、平台、最近活跃；本机标「本机」，别的设备可以解绑（二次确认）。
// 素笺云服务页与账号页共用它：文案里不出现「登录 / 账号」（约束 5）。

import SwiftUI

struct DeviceRow: View {
    let device: AccountDevice

    let revoke: () -> Void

    @State private var confirming = false

    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 2) {
                HStack(spacing: 6) {
                    Text(device.name)
                    if device.current {
                        Text("本机")
                            .font(AppFont.caption)
                            .foregroundStyle(.white)
                            .padding(.horizontal, 6)
                            .padding(.vertical, 1)
                            .background(.tint, in: Capsule())
                    }
                }
                Text(device.detail).font(AppFont.caption).foregroundStyle(.secondary)
            }
            Spacer()
            if !device.current {
                Button("解绑", role: .destructive) { confirming = true }
                    .buttonStyle(.borderless)
            }
        }
        .confirmationDialog("解绑「\(device.name)」？", isPresented: $confirming, titleVisibility: .visible) {
            Button("解绑", role: .destructive, action: revoke)
        } message: {
            Text("那台设备会退出素笺云服务，云功能随之停掉，要用时重新加入。")
        }
    }
}
