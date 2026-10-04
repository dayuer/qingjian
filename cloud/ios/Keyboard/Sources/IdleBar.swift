// 没在组字时的候选栏：私密输入框只亮一把锁；场景 / 对象卡面板打开时只留牌子与「完成」/「收起」；有别的设备刚复制的文字就提示它；
// 否则左边是场景牌子与「✨ 润色」，右边是「记一笔」与「发到其他设备」。润色进行中整栏交给 RewriteBar。
// 「记一笔」的确认条在提示行的位置（NoteBar），这一行的牌子照常在。

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
            } else if let offer = model.clipOffer {
                ClipOfferBar(model: model, offer: offer)
            } else if model.rewrite != .idle {
                RewriteBar(model: model)
            } else {
                actions
            }
        }
        .frame(height: KeyStyle.candidateBarHeight)
    }

    private var actions: some View {
        HStack(spacing: 0) {
            ScopeChip(model: model)
            if model.rewriteAvailable {
                Label("润色", systemImage: "sparkles")
                    .font(.system(size: 16))
                    .padding(.horizontal, 12)
                    .frame(maxHeight: .infinity)
                    .onKeyboardTap { model.startRewrite() }
            }
            Spacer()
            if model.canNote, model.noteDraft == nil, !model.noteDone {
                Label("记一笔", systemImage: "square.and.pencil")
                    .font(.system(size: 15))
                    .padding(.horizontal, 10)
                    .frame(maxHeight: .infinity)
                    .onKeyboardTap { model.startNote() }
            }
            if model.pasteboardChanged {
                Label("发到其他设备", systemImage: "arrow.up.doc.on.clipboard")
                    .font(.system(size: 15))
                    .padding(.horizontal, 10)
                    .frame(maxHeight: .infinity)
                    .onKeyboardTap { model.pushPasteboard() }
                Image(systemName: "xmark")
                    .font(.system(size: 13, weight: .medium))
                    .foregroundStyle(.secondary)
                    .frame(width: 40, height: KeyStyle.candidateBarHeight)
                    .onKeyboardPress { model.dismissPasteboard() }
                    .accessibilityLabel("不发送")
            }
        }
    }

    /// 场景选择与对象卡打开时的工具栏：牌子与右端的「完成」（选择面板）/「收起」（对象卡、没开完全访问）。
    private var panelBar: some View {
        HStack(spacing: 0) {
            ScopeChip(model: model)
            Spacer()
            Text(model.panel == .scope && model.fullAccess ? "完成" : "收起")
                .font(.system(size: 15, weight: .medium))
                .foregroundStyle(ScopeDisplay.panelCloseRole(fullAccess: model.fullAccess).color)
                .padding(.horizontal, 14)
                .frame(maxHeight: .infinity)
                .onKeyboardPress { model.closePanel() }
        }
    }
}
