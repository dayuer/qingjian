// 「待整理」里的一条（设计稿 .mem）：上面一行小字「时间 · 来源」（11.5 ink-3），中间原话（15，收起时两行），
// 下面一行 13 ink-2 的「展开全文 / 收起」，展开后多一个「删除」（删之前由 MaterialsSection 确认一次）。
// 展开后正文可以选中复制，所以整行不再挂点击手势——要点文字才展开。

import SwiftUI

struct MaterialRow: View {
    let material: MemoryMaterial

    let expanded: Bool

    /// 这条正在删：按钮置灰。
    let deleting: Bool

    let toggle: () -> Void

    let delete: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(MaterialDisplay.meta(at: material.at, source: material.source))
                .font(AppFont.font(size: 11.5))
                .foregroundStyle(Theme.ink3)
            Text(material.text)
                .font(AppFont.font(size: 15))
                .lineSpacing(3)
                .lineLimit(expanded ? nil : MaterialDisplay.previewLines)
                .textSelection(.enabled)
                .fixedSize(horizontal: false, vertical: true)
            HStack(spacing: 16) {
                Button(expanded ? MaterialDisplay.collapseHint : MaterialDisplay.expandHint, action: toggle)
                if expanded {
                    Button(MaterialDisplay.deleteButton, action: delete)
                        .disabled(deleting)
                        .opacity(deleting ? 0.4 : 1)
                }
            }
            .font(AppFont.font(size: 13))
            .foregroundStyle(Theme.ink2)
            .buttonStyle(.plain)
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 12)
        .frame(maxWidth: .infinity, alignment: .leading)
        .accessibilityAction(named: expanded ? MaterialDisplay.collapseHint : MaterialDisplay.expandHint, toggle)
    }
}
