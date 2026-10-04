// 点牌子后键区换成的选择面板：场景三选一；恋爱场景再选对象（App 里建的，最多 8 个，带头像与副文字）或不指定。
// 键盘扩展打不开 App，「新对象」只提示去 App 新建。没开完全访问时读不到 App Group 里的名单，也不让切场景（切了也用不上记忆），
// 面板里只有一句「开启完全访问后才能使用记忆」与「去开启」；桥不知道有没有完全访问，这道门在 Swift 侧。
// 「完成」/「收起」在候选栏那一行右端（IdleBar.panelBar）。

import SwiftUI

struct ScopePicker: View {
    let model: KeyboardModel

    @State private var showsNewContactTip = false

    private static let scenes = [MemoryScope.daily, MemoryScope.dating, MemoryScope.work]

    private let columns = Array(repeating: GridItem(.flexible(), spacing: 8), count: 4)

    var body: some View {
        switch ScopeDisplay.pickerMode(fullAccess: model.fullAccess) {
        case .picker: picker
        case .needsFullAccess: noAccess
        }
    }

    private var noAccess: some View {
        let style = NoFullAccessStyle.standard
        return VStack(spacing: 10) {
            Spacer(minLength: 0)
            Text(ScopeDisplay.needsFullAccessText)
                .font(.system(size: 15))
                .foregroundStyle(.secondary)
            Text("去开启")
                .font(.system(size: 15, weight: .medium))
                .foregroundStyle(style.buttonText.color)
                .padding(.horizontal, 14)
                .frame(height: 32)
                .overlay(Capsule().stroke(style.buttonStroke.color, lineWidth: 1))
                .onKeyboardPress { model.showNotice(ScopeDisplay.enableFullAccessNotice) }
            Text(model.notice ?? " ")
                .font(.system(size: 12))
                .foregroundStyle(style.notice.color)
            Spacer(minLength: 0)
        }
        .frame(maxWidth: .infinity)
    }

    private var picker: some View {
        VStack(alignment: .leading, spacing: 8) {
            HStack(spacing: 2) {
                ForEach(Self.scenes, id: \.self) { scene in
                    let selected = model.scope.scene == scene
                    Text(MemoryScope.title(of: scene))
                        .font(.system(size: 15, weight: selected ? .semibold : .regular))
                        .frame(maxWidth: .infinity, minHeight: 30)
                        .background(RoundedRectangle(cornerRadius: 7).fill(selected ? KeyStyle.keyFill : Color.clear))
                        .onKeyboardPress {
                            let keep = scene == model.scope.scene ? model.scope.contactId : nil
                            model.chooseScope(scene: scene, contactId: keep)
                        }
                }
            }
            .padding(2)
            .background(RoundedRectangle(cornerRadius: 9).fill(Color.secondary.opacity(0.15)))
            if model.scope.scene == MemoryScope.dating { contacts }
            Spacer(minLength: 0)
            Text("对象只能你自己切，键盘不知道你在和谁聊")
                .font(.system(size: 12))
                .foregroundStyle(.secondary)
        }
        .padding(.horizontal, 10)
        .padding(.vertical, 8)
    }

    @ViewBuilder
    private var contacts: some View {
        LazyVGrid(columns: columns, spacing: 6) {
            ForEach(model.contacts) { contact in
                cell(
                    avatar: contact.name, title: contact.name,
                    subtitle: ScopeDisplay.contactSubtitle(knownDays: contact.knownDays()),
                    selected: model.scope.contactId == contact.id
                ) {
                    model.chooseScope(scene: MemoryScope.dating, contactId: contact.id)
                }
            }
            cell(
                avatar: "–", title: "不指定", subtitle: ScopeDisplay.noScopeSubtitle,
                selected: model.scope.contactId == nil
            ) {
                model.chooseScope(scene: MemoryScope.dating, contactId: nil)
                model.closePanel()
            }
            cell(
                avatar: "+", title: "新对象",
                subtitle: showsNewContactTip ? "在素笺 App 里新建" : ScopeDisplay.newContactSubtitle(count: model.contacts.count),
                selected: false
            ) { showsNewContactTip = true }
        }
    }

    private func cell(
        avatar: String, title: String, subtitle: String, selected: Bool, action: @escaping () -> Void
    ) -> some View {
        HStack(spacing: 5) {
            MemoryAvatar(name: avatar, size: 24, selected: selected)
            VStack(alignment: .leading, spacing: 0) {
                Text(title).font(.system(size: 13, weight: .medium)).lineLimit(1)
                Text(subtitle)
                    .font(.system(size: 10))
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
                    .minimumScaleFactor(0.8)
            }
            Spacer(minLength: 0)
        }
        .padding(.horizontal, 6)
        .frame(maxWidth: .infinity, minHeight: 40)
        .background(RoundedRectangle(cornerRadius: 8).fill(KeyStyle.keyFill))
        .overlay(
            RoundedRectangle(cornerRadius: 8)
                .stroke(selected ? Theme.accent.color : Color.clear, lineWidth: 2)
        )
        .foregroundStyle(selected ? Theme.accentInk.color : Color.primary)
        .onKeyboardPress(action)
    }
}
