// 提示行的向上箭头打开的对象卡（设计稿 1b）：头像（56pt、衬线字）、名字（18pt 衬线）、「认识 n 天」，
// 今日相关最多 3 张卡放在白底圆角的一组里（左列日子：衬线灰绿字，3 天内写相对日子；喜好这类没有日期的写种类、灰字），
// 页脚左「只显示与今天有关的 n 条」、右「全部记忆」（灰绿）。「今早学习过」是云端整理后才有的状态，免费版不显示。
// 键盘扩展打不开 App，「全部记忆」只在页脚提示去 App 看；收起的向下箭头在工具栏右端（IdleBar.panelBar）。

import SwiftUI

struct ContactCardPanel: View {
    let model: KeyboardModel

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if let contact = model.currentContact {
                HStack(spacing: 12) {
                    MemoryAvatar(name: contact.chipName, size: 56, serif: true)
                    VStack(alignment: .leading, spacing: 4) {
                        Text(contact.chipName).font(SerifFont.font(size: 18, weight: .semibold))
                        Text(ScopeDisplay.contactSubtitle(knownDays: contact.knownDays()))
                            .font(.system(size: 12))
                            .foregroundStyle(Theme.ink3)
                    }
                }
                .padding(.bottom, 10)
                if model.panelCards.isEmpty {
                    Text("还没有记下这个人的事")
                        .font(.system(size: 14))
                        .foregroundStyle(Theme.ink3)
                } else {
                    cards
                }
            }
            Spacer(minLength: 0)
            HStack {
                Text(model.notice ?? ScopeDisplay.cardFooter(count: model.panelCards.count))
                    .font(.system(size: 12))
                    .foregroundStyle(model.notice == nil ? Theme.ink3 : ColorUsage.cardNotice.role.color)
                Spacer()
                Text("全部记忆")
                    .font(.system(size: 12))
                    .foregroundStyle(ColorUsage.allMemoryButton.role.color)
                    .padding(.leading, 10)
                    .frame(height: 24)
                    .onKeyboardPress { model.showNotice(ScopeDisplay.allMemoryNotice) }
            }
            .padding(.horizontal, 2)
        }
        .padding(.horizontal, 12)
        .padding(.top, 10)
        .padding(.bottom, 6)
    }

    private var cards: some View {
        VStack(spacing: 0) {
            ForEach(Array(model.panelCards.enumerated()), id: \.element.id) { index, card in
                if index > 0 { Divider().padding(.leading, 14) }
                row(card)
            }
        }
        .background(RoundedRectangle(cornerRadius: 14).fill(KeyStyle.keyFill))
        .overlay(RoundedRectangle(cornerRadius: 14).stroke(Color.primary.opacity(0.06), lineWidth: 1))
    }

    private func row(_ card: MemoryCard) -> some View {
        let date = card.dateLabel()
        return HStack(spacing: 12) {
            Text(date ?? card.kind.title)
                .font(SerifFont.font(size: 13, weight: .semibold))
                .foregroundStyle(date == nil ? Theme.ink3 : Theme.accentInk.color)
                .frame(width: 44)
            VStack(alignment: .leading, spacing: 1) {
                Text(card.text).font(.system(size: 15)).lineLimit(1)
                if !card.subtitle.isEmpty {
                    Text(card.subtitle).font(.system(size: 12.5)).foregroundStyle(Theme.ink3).lineLimit(1)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 8)
    }
}
