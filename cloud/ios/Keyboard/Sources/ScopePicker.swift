// 点牌子后键区换成的选择面板：场景三选一；恋爱场景再选对象（App 里建的，最多 8 个）或不指定。
// 键盘扩展打不开 App，「新对象」只提示去 App 新建。没开完全访问时读不到 App Group 里的名单，也不让切场景（切了也用不上记忆），
// 面板里只有一句「开启完全访问后才能使用记忆」；桥不知道有没有完全访问，这道门在 Swift 侧。

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
        VStack(spacing: 12) {
            Spacer(minLength: 0)
            Text(ScopeDisplay.needsFullAccessText)
                .font(.system(size: 15))
                .foregroundStyle(.secondary)
            Text("收起")
                .font(.system(size: 15, weight: .medium))
                .padding(.horizontal, 10)
                .frame(height: 32)
                .onKeyboardPress { model.closePanel() }
            Spacer(minLength: 0)
        }
        .frame(maxWidth: .infinity)
    }

    private var picker: some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack(spacing: 2) {
                ForEach(Self.scenes, id: \.self) { scene in
                    let selected = model.scope.scene == scene
                    Text(MemoryScope.title(of: scene))
                        .font(.system(size: 15, weight: selected ? .semibold : .regular))
                        .frame(maxWidth: .infinity, minHeight: 32)
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
            HStack {
                Text("对象只能你自己切，键盘不知道你在和谁聊")
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
        .padding(.horizontal, 10)
        .padding(.vertical, 8)
    }

    @ViewBuilder
    private var contacts: some View {
        LazyVGrid(columns: columns, spacing: 8) {
            ForEach(model.contacts) { contact in
                cell(contact.name, selected: model.scope.contactId == contact.id) {
                    model.chooseScope(scene: MemoryScope.dating, contactId: contact.id)
                }
            }
            cell("不指定", selected: model.scope.contactId == nil) {
                model.chooseScope(scene: MemoryScope.dating, contactId: nil)
                model.closePanel()
            }
            cell("＋ 新对象", selected: false) { showsNewContactTip = true }
        }
        if showsNewContactTip {
            Text("在素笺 App 里新建")
                .font(.system(size: 13))
                .foregroundStyle(Color.accentColor)
        }
    }

    private func cell(_ title: String, selected: Bool, action: @escaping () -> Void) -> some View {
        Text(title)
            .font(.system(size: 14))
            .lineLimit(1)
            .frame(maxWidth: .infinity, minHeight: 34)
            .background(
                RoundedRectangle(cornerRadius: 8).fill(selected ? Color.accentColor.opacity(0.18) : KeyStyle.keyFill))
            .foregroundStyle(selected ? Color.accentColor : Color.primary)
            .onKeyboardPress(action)
    }
}
