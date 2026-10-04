// 没在组字时的工具栏（设计稿 .k-bar）：私密输入框只亮一把锁；场景选择打开时是牌子加「完成」，对象卡打开时是不拆开的牌子加向下箭头；
// 有别的设备刚复制的文字就提示它；否则左边牌子，右边依次「发到其他设备」（本机剪贴板变了才有）「记一笔」「改写」（配了素笺云才有）和收起键盘的向下箭头。
// 点了牌子右半时，右边这些让出位置，横列本场景的其他人与「不指定」。改写进行中整栏交给 RewriteBar。
// 「记一笔」的确认条与手写条在提示行的位置（NoteBar / NoteComposeBar），这一行的牌子照常在；
// 手写时只留牌子：改写、插入剪贴板、发到其他设备都是对宿主的操作，此时不该出。

import SwiftUI

struct IdleBar: View {
    let model: KeyboardModel

    var body: some View {
        Group {
            if model.privateField {
                Label("隐私输入：不学习、不上传", systemImage: "lock.fill")
                    .font(.system(size: 13))
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity)
            } else if model.panel == .scope || model.panel == .contactCard {
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
            } else {
                actions
            }
        }
        .frame(height: KeyStyle.candidateBarHeight)
        .overlay(alignment: .bottom) { Rectangle().fill(Color.primary.opacity(0.08)).frame(height: 1) }
    }

    private var actions: some View {
        HStack(spacing: 0) {
            ScopeChip(model: model)
            Spacer(minLength: 4)
            if model.pasteboardChanged {
                tool("发到其他设备") { model.pushPasteboard() }
                Image(systemName: "xmark")
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(Theme.ink3)
                    .frame(width: 24, height: KeyStyle.candidateBarHeight)
                    .onKeyboardPress { model.dismissPasteboard() }
                    .accessibilityLabel("不发送")
            }
            if model.canNote, model.noteDraft == nil, !model.noteDone {
                tool("记一笔") { model.startNote() }
            }
            if model.rewriteAvailable {
                tool("改写") { model.startRewrite() }
            }
            ToolbarArrow(up: false, label: "收起键盘") { model.dismissKeyboard() }
                .padding(.trailing, 2)
        }
    }

    /// 牌子右半展开的人：白胶囊（设计稿 .chip），点一个就切过去并收起。
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

    /// 选择面板与对象卡打开时的工具栏。
    private var panelBar: some View {
        HStack(spacing: 0) {
            ScopeChip(model: model, split: model.panel == .scope, interactive: false)
            Spacer()
            if model.panel == .contactCard {
                ToolbarArrow(up: false, label: "收起对象卡") { model.closePanel() }
                    .padding(.trailing, 2)
            } else {
                tool(model.fullAccess ? "完成" : "收起") { model.closePanel() }
                    .padding(.trailing, 4)
            }
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
