// 「待整理」里的一条：上面一行小字「时间 · 来源」（ink-3），下面原话（15pt，原样，名字与时间行都在）。
// 收起时只显示前 2 行（MaterialDisplay.preview），点一下展开全文、可以选中复制；展开后右下角有「删除」，删之前由 MaterialsSection 确认一次。

import SwiftUI

struct MaterialRow: View {
    let material: MemoryMaterial

    let expanded: Bool

    /// 这条正在删：删除按钮置灰。
    let deleting: Bool

    let toggle: () -> Void

    let delete: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            Text(MaterialDisplay.meta(at: material.at, source: material.source))
                .font(AppFont.font(size: 11.5))
                .foregroundStyle(Theme.ink3)
            if expanded {
                Text(material.text)
                    .font(AppFont.font(size: 15))
                    .lineSpacing(3)
                    .textSelection(.enabled)
                    .fixedSize(horizontal: false, vertical: true)
                HStack {
                    Text(MaterialDisplay.collapseHint)
                        .font(AppFont.font(size: 12.5))
                        .foregroundStyle(Theme.ink3)
                    Spacer()
                    Button(MaterialDisplay.deleteButton, action: delete)
                        .font(AppFont.font(size: 13, weight: .medium))
                        .foregroundStyle(ColorUsage.materialDelete.role.color)
                        .buttonStyle(.borderless)
                        .disabled(deleting)
                        .opacity(deleting ? 0.4 : 1)
                }
            } else {
                Text(MaterialDisplay.preview(material.text))
                    .font(AppFont.font(size: 15))
                    .lineSpacing(3)
                    .lineLimit(MaterialDisplay.previewLines)
                if MaterialDisplay.isLong(material.text) {
                    Text(MaterialDisplay.expandHint)
                        .font(AppFont.font(size: 12.5))
                        .foregroundStyle(Theme.ink3)
                }
            }
        }
        .padding(.vertical, 2)
        .frame(maxWidth: .infinity, alignment: .leading)
        .contentShape(Rectangle())
        .onTapGesture(perform: toggle)
        .swipeActions(edge: .trailing, allowsFullSwipe: false) {
            Button(MaterialDisplay.deleteButton, role: .destructive, action: delete)
        }
        .accessibilityAction(named: expanded ? MaterialDisplay.collapseHint : MaterialDisplay.expandHint, toggle)
    }
}
