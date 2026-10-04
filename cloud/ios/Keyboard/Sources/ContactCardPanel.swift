// 提示行「展开」后键区换成的对象卡：头像、名字、认识几天、今日相关最多 3 张卡（左列相对日子，右边标题加小字），页脚是数量说明与「全部记忆」。
// 键盘扩展打不开 App，「全部记忆」只在面板里提示去 App 看；「收起」在候选栏那一行右端（IdleBar.panelBar）。

import SwiftUI

struct ContactCardPanel: View {
    let model: KeyboardModel

    var body: some View {
        VStack(alignment: .leading, spacing: 8) {
            if let contact = model.currentContact {
                HStack(spacing: 10) {
                    MemoryAvatar(name: contact.name, size: 38, selected: true)
                    VStack(alignment: .leading, spacing: 2) {
                        Text(contact.name).font(.system(size: 17, weight: .semibold))
                        Text(ScopeDisplay.contactSubtitle(knownDays: contact.knownDays()))
                            .font(.system(size: 12))
                            .foregroundStyle(.secondary)
                    }
                }
                if model.panelCards.isEmpty {
                    Text("还没有记下这个人的事")
                        .font(.system(size: 14))
                        .foregroundStyle(.secondary)
                }
                ForEach(model.panelCards) { card in
                    HStack(alignment: .firstTextBaseline, spacing: 8) {
                        Text(card.dateLabel() ?? card.kind.title)
                            .font(.system(size: 12))
                            .foregroundStyle(card.dateLabel() == nil ? Color.secondary : Theme.accentInk.color)
                            .frame(width: 40, alignment: .leading)
                        VStack(alignment: .leading, spacing: 0) {
                            Text(card.text).font(.system(size: 15)).lineLimit(1)
                            if !card.subtitle.isEmpty {
                                Text(card.subtitle).font(.system(size: 11)).foregroundStyle(.secondary).lineLimit(1)
                            }
                        }
                    }
                }
            }
            Spacer(minLength: 0)
            HStack {
                Text(model.notice ?? ScopeDisplay.cardFooter(count: model.panelCards.count))
                    .font(.system(size: 12))
                    .foregroundStyle(model.notice == nil ? Color.secondary : Theme.accentInk.color)
                Spacer()
                Text("全部记忆")
                    .font(.system(size: 14, weight: .medium))
                    .foregroundStyle(Theme.accentInk.color)
                    .padding(.horizontal, 10)
                    .frame(height: 28)
                    .onKeyboardPress { model.showNotice(ScopeDisplay.allMemoryNotice) }
            }
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 8)
    }
}
