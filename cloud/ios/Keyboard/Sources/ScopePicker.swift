// 点牌子左半后键区换成的选择面板（设计稿 1d）：场景分段三选一，切场景回到这个场景上次选的人；
// 下面 4 列格子是本场景的人（副文字是上次用的时间，选中的灰绿描边、头像换灰绿底；工作场景用中性色）、「不指定」与「新对象 n/8」。
// 人少时照设计稿竖排格子，多了换横排矮格子（ScopeDisplay.cellStyle）。「新对象」在提示行的位置打名字直接建在当前场景
// （KeyboardModel.startNamingContact；设计稿写的是跳 App，用户要在键盘里建），称呼先按 TA，详细的在 App 里补。
// 没开完全访问时读不到 App Group 里的名单，也不让切场景，面板里只有一句说明与「去开启」（点了在面板里展开设置路径，不跳转）；桥不知道有没有完全访问，这道门在 Swift 侧。
// 「完成」在工具栏右端（IdleBar.panelBar）。

import SwiftUI

struct ScopePicker: View {
    let model: KeyboardModel

    /// 没开完全访问时点了「去开启」，展开设置路径；再点收起。
    @State private var showsPath = false

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
                .foregroundStyle(style.explanation.color)
                .multilineTextAlignment(.center)
            Text("去开启")
                .font(.system(size: 15, weight: .medium))
                .foregroundStyle(style.buttonText.color)
                .padding(.horizontal, 14)
                .frame(height: 32)
                .overlay(Capsule().stroke(style.buttonStroke.color, lineWidth: 1))
                .onKeyboardPress { showsPath.toggle() }
                .accessibilityAddTraits(.isButton)
            // 不跳转：键盘扩展打不开系统设置，也不许借响应链打开 App，只展开路径文字；占位保持高度不跳。
            Text(ScopeDisplay.fullAccessPath)
                .font(.system(size: 13))
                .foregroundStyle(style.path.color)
                .multilineTextAlignment(.center)
                .opacity(showsPath ? 1 : 0)
                .accessibilityHidden(!showsPath)
            Spacer(minLength: 0)
        }
        .padding(.horizontal, 16)
        .frame(maxWidth: .infinity)
    }

    private var picker: some View {
        VStack(alignment: .leading, spacing: 10) {
            segments
            LazyVGrid(columns: columns, spacing: 8) {
                ForEach(model.people) { contact in
                    cell(
                        avatar: contact.name, title: contact.name,
                        subtitle: ScopeDisplay.lastUsed(at: model.scope.used[contact.id]),
                        selected: model.scope.contactId == contact.id
                    ) { model.chooseContact(contact.id) }
                }
                cell(
                    avatar: "不", title: ScopeDisplay.noScopeTitle, subtitle: ScopeDisplay.noScopeSubtitle,
                    selected: model.scope.contactId == nil
                ) { model.chooseContact(nil) }
                addCell
            }
            Spacer(minLength: 0)
            Text(model.notice ?? "对象只能你自己切，键盘不知道你在和谁聊")
                .font(.system(size: 11.5))
                .foregroundStyle(Theme.ink3)
        }
        .padding(.horizontal, 12)
        .padding(.top, 10)
        .padding(.bottom, 6)
    }

    /// 设计稿 .seg：灰底里三段，选中的白底。
    private var segments: some View {
        HStack(spacing: 0) {
            ForEach(MemoryScope.pickerOrder, id: \.self) { scene in
                let selected = model.scope.scene == scene
                Text(MemoryScope.title(of: scene))
                    .font(.system(size: 13, weight: selected ? .medium : .regular))
                    .foregroundStyle(selected ? Theme.ink : Theme.ink2)
                    .frame(maxWidth: .infinity, minHeight: 28)
                    .background(
                        RoundedRectangle(cornerRadius: 7)
                            .fill(selected ? KeyStyle.keyFill : Color.clear)
                            .shadow(color: .black.opacity(selected ? 0.1 : 0), radius: 1, y: 1))
                    .onKeyboardPress { model.chooseScene(scene) }
            }
        }
        .padding(2)
        .background(RoundedRectangle(cornerRadius: 9).fill(Color.secondary.opacity(0.2)))
    }

    private var tall: Bool { ScopeDisplay.cellStyle(people: model.people.count) == .tall }

    private func cell(
        avatar: String, title: String, subtitle: String, selected: Bool, action: @escaping () -> Void
    ) -> some View {
        let scene = model.scope.scene
        return layout {
            MemoryAvatar(
                name: avatar, size: tall ? 34 : 24, selected: selected, scene: scene, serif: true,
                accentOnlyWhenSelected: true)
            labels(title: title, subtitle: subtitle)
        }
        .background(RoundedRectangle(cornerRadius: 10).fill(KeyStyle.keyFill))
        .shadow(color: .black.opacity(0.1), radius: 0, y: 1)
        .overlay(
            RoundedRectangle(cornerRadius: 10)
                .stroke(selected ? selectionStroke(scene) : Color.clear, lineWidth: 1.5)
        )
        .onKeyboardPress(action)
        .accessibilityAddTraits(selected ? [.isButton, .isSelected] : .isButton)
    }

    /// 设计稿 .cell.on 的描边是 accent 实色（不是 accent-ink）；工作场景用中性色。
    private func selectionStroke(_ scene: String) -> Color {
        MemoryScope.usesAccent(scene) ? Theme.accent.color : ColorUsage.selectedContactCell.role(in: scene).color
    }

    private var addCell: some View {
        layout {
            Text("+")
                .font(.system(size: tall ? 18 : 15))
                .frame(width: tall ? 34 : 24, height: tall ? 34 : 24)
            labels(title: "新对象", subtitle: ScopeDisplay.newContactSubtitle(count: model.people.count))
        }
        .foregroundStyle(Theme.ink3)
        .overlay(RoundedRectangle(cornerRadius: 10).stroke(Color.primary.opacity(0.12), lineWidth: 1))
        .onKeyboardPress { model.startNamingContact() }
        .accessibilityAddTraits(.isButton)
    }

    @ViewBuilder
    private func layout<Content: View>(@ViewBuilder _ content: () -> Content) -> some View {
        if tall {
            VStack(spacing: 4) { content() }
                .frame(maxWidth: .infinity)
                .padding(.top, 9)
                .padding(.bottom, 7)
        } else {
            HStack(spacing: 5) {
                content()
                Spacer(minLength: 0)
            }
            .padding(.horizontal, 6)
            .frame(maxWidth: .infinity, minHeight: 40)
        }
    }

    private func labels(title: String, subtitle: String) -> some View {
        VStack(alignment: tall ? .center : .leading, spacing: tall ? 2 : 0) {
            Text(title).font(.system(size: 13)).lineLimit(1)
            Text(subtitle)
                .font(.system(size: 10.5))
                .foregroundStyle(Theme.ink3)
                .lineLimit(1)
                .minimumScaleFactor(0.8)
        }
    }
}
