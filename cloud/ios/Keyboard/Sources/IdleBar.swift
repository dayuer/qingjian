// 没在组字时的工具栏（设计稿 .k-bar）：私密输入框只亮一把锁；对象卡打开时是不拆开的牌子加向下箭头；
// 有别的设备刚复制的文字就提示它；否则左边牌子，右边依次「发到其他设备」（本机剪贴板变了才有）「记一笔」「改写」（配了素笺云才有）和收起键盘的向下箭头。
// 点了牌子时，右边这些让出位置，横列其他人与「不指定」。改写进行中整栏交给 RewriteBar。
// 没开完全访问时牌子是说明入口（ScopeChip.fullAccessNote），点开在这一栏里展开整句与开启路径（键盘扩展打不开系统设置，只能给文字）。
// 「记一笔」的确认条与手写条在提示行的位置（NoteBar / NoteComposeBar），这一行的牌子照常在；
// 手写时只留牌子：改写、插入剪贴板、发到其他设备都是对宿主的操作，此时不该出。

import SwiftUI

struct IdleBar: View {
    let model: KeyboardModel

    /// 说明展开后又点了「去开启」：那行换成设置路径。
    @State private var showsFullAccessPath = false

    var body: some View {
        Group {
            if model.privateField {
                Label("隐私输入：不学习、不上传", systemImage: "lock.fill")
                    .font(.system(size: 13))
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity)
            } else if !model.fullAccess, model.showsFullAccessNote {
                fullAccessNote
            } else if model.panel == .contactCard {
                panelBar
            } else if model.composedNote != nil {
                HStack(spacing: 0) {
                    ScopeChip(model: model, interactive: false)
                    Spacer()
                }
            } else if let offer = model.clipOffer {
                ClipOfferBar(model: model, offer: offer)
            } else if model.rewrite != .idle {
                RewriteBar(model: model)
            } else if model.quickOpen {
                quickPicks
            } else if model.showsRewriteSkills {
                RewriteSkillRow(
                    skills: model.rewriteSkills, current: model.rewriteSkillPick,
                    onPick: { model.pickRewriteSkill($0) })
            } else {
                actions
            }
        }
        .frame(height: KeyStyle.candidateBarHeight)
        .overlay(alignment: .bottom) { Rectangle().fill(Color.primary.opacity(0.08)).frame(height: 1) }
        // 收起来再点开时从整句重新看起，不留在「去开启」那份路径上
        .onChange(of: model.showsFullAccessNote) { _, expanded in
            if !expanded { showsFullAccessPath = false }
        }
    }

    /// 展开的说明：整句（牌子上只放得下一行）+「去开启」；点「去开启」就地换成设置路径，不跳转（键盘扩展打不开系统设置）。
    /// 点文字本身把它收回去。
    private var fullAccessNote: some View {
        let style = NoFullAccessStyle.standard
        return HStack(spacing: 8) {
            Text(showsFullAccessPath ? ScopeDisplay.fullAccessPath : ScopeDisplay.needsFullAccessText)
                .font(.system(size: 12.5))
                .foregroundStyle(showsFullAccessPath ? style.path.color : style.explanation.color)
                .lineLimit(2)
                .minimumScaleFactor(0.85)
                .contentShape(Rectangle())
                .onKeyboardPress { model.toggleFullAccessNote() }
                .accessibilityAddTraits(.isButton)
            Spacer(minLength: 4)
            Text("去开启")
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(style.buttonText.color)
                .padding(.horizontal, 14)
                .frame(height: 30)
                .overlay(Capsule().stroke(style.buttonStroke.color, lineWidth: 1))
                .onKeyboardPress { showsFullAccessPath.toggle() }
                .accessibilityAddTraits(.isButton)
            // 这一栏把 actions 整行换掉了，收起键盘的箭头得跟着挪过来，不然用户没法收键盘
            ToolbarArrow(up: false, label: "收起键盘") { model.dismissKeyboard() }
                .padding(.trailing, 2)
        }
        .padding(.horizontal, 12)
    }

    private var actions: some View {
        HStack(spacing: 0) {
            ScopeChip(model: model)
            Spacer(minLength: 4)
            if model.canNote, model.noteDraft == nil, !model.noteDone {
                tool("记一笔") { model.startNote() }
            }
            if model.rewriteAvailable {
                // 按钮写着当前技能的（没读到技能表时为「改写」）；点一下用技能名右边的样子展开技能排
                tool(model.rewriteSkill?.name ?? "改写") { model.toggleRewriteSkills() }
            }
            ToolbarArrow(up: false, label: "收起键盘") { model.dismissKeyboard() }
                .padding(.trailing, 2)
        }
    }

    /// 牌子展开的人：白胶囊（设计稿 .chip），点一个就切过去并收起。
    private var quickPicks: some View {
        HStack(spacing: 6) {
            ScopeChip(model: model)
            HStack(spacing: 6) {
                ForEach(model.quickPicks, id: \.self) { id in
                    let name = id.flatMap { id in model.contacts.first { $0.id == id }?.chipName }
                    Text(name ?? ScopeDisplay.noScopeTitle)
                        .font(.system(size: 13, weight: .medium))
                        .foregroundStyle(Theme.ink)
                        .lineLimit(1)
                        .padding(.leading, 9)
                        .padding(.trailing, 11)
                        .frame(height: 30)
                        .background(Capsule().fill(KeyStyle.keyFill))
                        .shadow(color: .black.opacity(0.08), radius: 0, y: 1)
                        .onKeyboardPress { model.chooseContact(id) }
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            .clipped()
        }
        .padding(.trailing, 8)
    }

    /// 对象卡打开时的工具栏：不拆开的牌子加「收起对象卡」。
    private var panelBar: some View {
        HStack(spacing: 0) {
            ScopeChip(model: model, interactive: false)
            Spacer()
            ToolbarArrow(up: false, label: "收起对象卡") { model.closePanel() }
                .padding(.trailing, 2)
        }
    }

    /// 设计稿 .tool：13pt、ink-2 的文字按钮。
    private func tool(_ title: String, action: @escaping () -> Void) -> some View {
        Text(title)
            .font(.system(size: 13))
            .foregroundStyle(ColorUsage.panelDone.role.color)
            .padding(.horizontal, 10)
            .frame(maxHeight: .infinity)
            .onKeyboardPress(action)
            .accessibilityAddTraits(.isButton)
    }
}
