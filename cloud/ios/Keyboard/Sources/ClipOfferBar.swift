// 别的设备刚复制的文字：点一下插入，✕ 不再提示。

import SwiftUI

struct ClipOfferBar: View {
    let model: KeyboardModel

    let offer: ClipOffer

    var body: some View {
        HStack(spacing: 6) {
            Image(systemName: "doc.on.clipboard").foregroundStyle(.secondary).padding(.leading, 12)
            Text(offer.device).font(.system(size: 13)).foregroundStyle(.secondary)
            Text(offer.text.replacingOccurrences(of: "\n", with: " "))
                .font(.system(size: 17))
                .foregroundStyle(Theme.ink)
                .lineLimit(1)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
                .onKeyboardTap { model.insertClip() }
            Image(systemName: "xmark")
                .font(.system(size: 15, weight: .medium))
                .foregroundStyle(.secondary)
                .frame(width: 44, height: KeyStyle.candidateBarHeight)
                .onKeyboardPress { model.dismissClip() }
                .accessibilityLabel("不插入")
        }
        .accessibilityElement(children: .contain)
        .accessibilityLabel("来自 \(offer.device) 的剪贴板")
    }
}
