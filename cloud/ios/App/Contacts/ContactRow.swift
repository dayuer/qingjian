// 通讯录的一行（设计稿 02 的 2b）：17pt 名字、行尾灰绿的事件提示，行下一条 0.5pt 细线。
// 点一下在下面**展开**这个人的几张记忆卡（2026-10-05 定的「通栏列表 + 行内展开」：通讯录也要能看见记着的事，
// 不用另翻一页），展开区末尾一行进对象详情——那里才是改卡、看全部的地方。再点一行收起。

import SwiftUI

struct ContactRow: View {
    let contact: MemoryContact

    /// 行尾提示（「明天生日」）；没有就留空。
    let note: String?

    /// 这个人的全部记忆卡；展开后显示最近改过的几张，页脚写总数（不是显示的条数）。
    let cards: [MemoryCard]

    let expanded: Bool

    let onToggle: () -> Void

    let onOpenDetail: () -> Void

    var body: some View {
        VStack(spacing: 0) {
            Button(action: onToggle) { row }
                .buttonStyle(.plain)
                .accessibilityIdentifier("contact-\(contact.id)")
            if expanded { detail }
        }
    }

    private var row: some View {
        HStack(spacing: 8) {
            Text(contact.name)
                .font(AppFont.font(size: 17))
                .foregroundStyle(Theme.ink)
            Spacer(minLength: 8)
            if let note {
                Text(note)
                    .font(AppFont.font(size: 13))
                    .foregroundStyle(ColorUsage.contactEventNote.role.color)
                    .lineLimit(1)
            }
        }
        .frame(height: 44)
        // 细线在内外边距里面：设计稿的 `.line` 从 20 画到右边距 24，不是通到屏幕两边
        .overlay(alignment: .bottom) { hairline }
        .padding(.leading, 20)
        .padding(.trailing, 24)
        .contentShape(Rectangle())
    }

    private var detail: some View {
        VStack(spacing: 0) {
            if cards.isEmpty {
                Text("还没有记着的")
                    .font(AppFont.font(size: 13))
                    .foregroundStyle(Theme.ink3)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.leading, 34)
                    .padding(.trailing, 24)
                    .frame(height: 36)
            } else {
                ForEach(ContactIndex.preview(cards)) { card in
                    HStack(spacing: 8) {
                        Text(card.text)
                            .font(AppFont.font(size: 15))
                            .foregroundStyle(Theme.ink)
                            .lineLimit(1)
                        Spacer(minLength: 8)
                        Text(card.kind.title)
                            .font(AppFont.font(size: 11))
                            .foregroundStyle(Theme.ink3)
                    }
                    .padding(.leading, 34)
                    .padding(.trailing, 24)
                    .frame(height: 36)
                }
            }
            Button(action: onOpenDetail) {
                HStack(spacing: 4) {
                    Text("全部 \(cards.count) 条记忆")
                    Image(systemName: "chevron.right")
                        .font(AppFont.font(size: 11, weight: .semibold))
                }
                .font(AppFont.font(size: 13, weight: .medium))
                .foregroundStyle(ColorUsage.allMemoryButton.role.color)
                .padding(.leading, 34)
                .padding(.trailing, 24)
                .frame(maxWidth: .infinity, alignment: .leading)
                .frame(height: 40)
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
        }
        // 展开区整块比列表底色浅一档，一眼看出这几行属于上面那个人
        .background(Color(.secondarySystemBackground).opacity(0.6))
        .overlay(alignment: .bottom) { hairline }
    }

    private var hairline: some View {
        Rectangle()
            .fill(Hairline.line)
            .frame(height: 0.5)
    }
}
