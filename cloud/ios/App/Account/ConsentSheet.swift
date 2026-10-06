// 开「云端记忆」与「同步打字内容」前的同意说明：写明数据去哪、交给谁、随时可停。
// 文案由审计定（2026-10-06）：供应商 DeepSeek、素材与日志存新加坡、用于优化时数据经新加坡发往中国境内。

import SwiftUI

/// 需要先过同意说明的功能；别的功能返回 nil（开关直接生效）。
enum ConsentCopy {
    static func of(_ feature: CloudFeature) -> (title: String, points: [String])? {
        switch feature {
        case .memory:
            (
                "云端记忆",
                [
                    "记下的素材先在本机抹去姓名、电话、地址等，再上传到素笺的服务器（新加坡）。",
                    "整理交给 DeepSeek：数据经新加坡发往中国境内处理，对方不保存、不拿来训练，整理完原文即删。",
                    "随时可以关掉；关掉会同时删除服务器上的素材。",
                ]
            )
        case .inputLog:
            (
                "同步打字内容",
                [
                    "打的字会上传到素笺的服务器（新加坡），用来优化输入法。",
                    "用于 AI 优化时会先脱敏再发给 DeepSeek（数据在中国境内处理）。",
                    "随时可以关掉，也可以一键清空云端记录；密码、验证码这类输入框不会记录。",
                ]
            )
        default:
            nil
        }
    }
}

/// 同意页：勾选「我同意」之前「同意并开启」不可用。
struct ConsentSheet: View {
    let feature: CloudFeature

    let onAgree: () -> Void

    @Environment(\.dismiss) private var dismiss

    @State private var agreed = false

    var body: some View {
        NavigationStack {
            Form {
                if let copy = ConsentCopy.of(feature) {
                    Section {
                        Text(copy.title)
                            .font(.headline)
                    }
                    Section("你需要知道") {
                        ForEach(copy.points, id: \.self) { point in
                            Label(point, systemImage: "circle.fill")
                                .font(.footnote)
                                
                                .foregroundStyle(.secondary)
                        }
                        .listRowBackground(Color.clear)
                    }
                    Section {
                        Toggle("我已阅读并同意以上说明", isOn: $agreed)
                            .tint(ColorUsage.appToggle.role.color)
                    }
                }
            }
            .navigationTitle("开启前的说明")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button("取消") { dismiss() }
                }
                ToolbarItem(placement: .confirmationAction) {
                    Button("同意并开启") {
                        onAgree()
                        dismiss()
                    }
                    .disabled(!agreed)
                }
            }
        }
        .presentationDetents([.medium, .large])
    }
}
