// 「忘掉这个人」的底部弹层（02 的 1d）：22pt「忘掉 X？」、「n 天里的 n 条记忆…」、黑底「忘掉」、「再想想」。
// 这里只回报选了什么；真正的写入由 ContactSettingsView 在 .sheet 的 onDismiss 里做（弹层收起后才弹得出失败提示）。

import SwiftUI

struct ForgetContactSheet: View {
    let name: String

    let knownDays: Int

    let cardCount: Int

    /// 点了「忘掉」：调用方记下来，并收起弹层。
    let onForget: () -> Void

    @Environment(\.dismiss) private var dismiss

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text(MemoryDetailText.forgetTitle(name: name))
                .font(AppFont.font(size: 22, weight: .semibold))
                .foregroundStyle(Theme.ink)
            Text(MemoryDetailText.forgetBody(knownDays: knownDays, cardCount: cardCount))
                .font(AppFont.font(size: 13.5))
                .lineSpacing(4)
                .foregroundStyle(Theme.ink2)
                .fixedSize(horizontal: false, vertical: true)
            Button {
                onForget()
                dismiss()
            } label: {
                Text("忘掉")
                    .font(AppFont.font(size: 16, weight: .medium))
                    .foregroundStyle(Color(UIColor.systemBackground))
                    .frame(maxWidth: .infinity)
                    .frame(height: 50)
                    .background(Capsule().fill(Theme.ink))
            }
            .buttonStyle(.plain)
            Button {
                dismiss()
            } label: {
                Text("再想想")
                    .font(AppFont.font(size: 16, weight: .medium))
                    .foregroundStyle(Theme.ink2)
                    .frame(maxWidth: .infinity)
                    .frame(height: 50)
            }
            .buttonStyle(.plain)
        }
        .padding(.horizontal, 20)
        .padding(.top, 24)
        .presentationDetents([.height(300)])
        .presentationDragIndicator(.visible)
    }
}
