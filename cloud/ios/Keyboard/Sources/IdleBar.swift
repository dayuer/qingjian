// 没在组字时的候选栏：私密输入框只亮一把锁；有别的设备刚复制的文字就提示它；「记一笔」待确认时是确认条；
// 否则左边是场景牌子与「✨ 润色」，右边是「记一笔」与「发到其他设备」。润色进行中整栏交给 RewriteBar。

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
            } else if let offer = model.clipOffer {
                ClipOfferBar(model: model, offer: offer)
            } else if let draft = model.noteDraft {
                noteConfirm(draft)
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
            if model.canNote {
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

    /// 「记一笔」的确认条：剪贴板里的字、「记到 {对象}」、「忽略」。
    private func noteConfirm(_ draft: String) -> some View {
        HStack(spacing: 8) {
            Text(draft.replacingOccurrences(of: "\n", with: " "))
                .font(.system(size: 14))
                .foregroundStyle(.secondary)
                .lineLimit(1)
                .padding(.leading, 12)
                .frame(maxWidth: .infinity, alignment: .leading)
            Text("记到 \(model.currentContact?.name ?? "")")
                .font(.system(size: 15, weight: .medium))
                .foregroundStyle(Color.accentColor)
                .padding(.horizontal, 10)
                .frame(maxHeight: .infinity)
                .onKeyboardPress { model.confirmNote() }
            Text("忽略")
                .font(.system(size: 15))
                .foregroundStyle(.secondary)
                .padding(.trailing, 12)
                .frame(maxHeight: .infinity)
                .onKeyboardPress { model.cancelNote() }
        }
    }
}
