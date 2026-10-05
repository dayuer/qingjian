// 首页顶上常驻的一行：记忆读不出来（没有 App Group、锁着读不到）时说清楚原因，并给一个「再读一次」。不让列表静默地空着。

import SwiftUI

struct MemoryFailureBanner: View {
    let text: String

    let retry: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            Label(text, systemImage: "exclamationmark.triangle")
                .foregroundStyle(ColorUsage.failureBanner.role.color)
            Button("再读一次", action: retry)
                .font(AppFont.subheadline.weight(.medium))
                .foregroundStyle(ColorUsage.failureBanner.role.color)
        }
        .padding(.vertical, 4)
    }
}
