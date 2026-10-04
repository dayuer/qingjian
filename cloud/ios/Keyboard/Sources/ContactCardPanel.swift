// 提示行「展开」后键区换成的对象卡：头像字、名字、认识几天、今日相关最多 3 张卡。键盘扩展打不开 App，全部记忆只提示去 App 看。

import SwiftUI

struct ContactCardPanel: View {
    let model: KeyboardModel

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            if let contact = model.currentContact {
                HStack(spacing: 10) {
                    MemoryAvatar(name: contact.name, size: 40)
                    VStack(alignment: .leading, spacing: 2) {
                        Text(contact.name).font(.system(size: 17, weight: .semibold))
                        Text("认识 \(contact.knownDays()) 天")
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
                        Text(card.kind.title)
                            .font(.system(size: 12))
                            .foregroundStyle(.secondary)
                            .frame(width: 32, alignment: .leading)
                        Text(card.text)
                            .font(.system(size: 15))
                            .lineLimit(2)
                    }
                }
            }
            Spacer(minLength: 0)
            HStack {
                Text("全部记忆在素笺 App 里")
                    .font(.system(size: 12))
                    .foregroundStyle(.secondary)
                Spacer()
                Text("收起")
                    .font(.system(size: 15, weight: .medium))
                    .padding(.horizontal, 10)
                    .frame(height: 32)
                    .onKeyboardPress { model.closePanel() }
            }
        }
        .padding(.horizontal, 14)
        .padding(.vertical, 10)
    }
}
