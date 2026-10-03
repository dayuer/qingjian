// 没在组字时的候选栏：私密输入框只亮一把锁；有别的设备刚复制的文字就提示它；
// 否则是「✨ 润色」，本机剪贴板变过时右边再给「发到其他设备」。润色进行中整栏交给 RewriteBar。

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
            if model.rewriteAvailable {
                Label("润色", systemImage: "sparkles")
                    .font(.system(size: 16))
                    .padding(.horizontal, 12)
                    .frame(maxHeight: .infinity)
                    .onKeyboardTap { model.startRewrite() }
            }
            Spacer()
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
}
