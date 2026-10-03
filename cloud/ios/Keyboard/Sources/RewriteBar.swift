// 没在组字时的候选栏：「✨ 润色」光标前的一段话，结果出来后点一下替换原文，✕ 放弃。

import SwiftUI

struct RewriteBar: View {
    let model: KeyboardModel

    var body: some View {
        HStack(spacing: 8) {
            switch model.rewrite {
            case .idle:
                Label("润色", systemImage: "sparkles")
                    .font(.system(size: 16))
                    .padding(.horizontal, 12)
                    .frame(maxHeight: .infinity)
                    .onKeyboardTap { model.startRewrite() }
                Spacer()
            case .pending:
                ProgressView().padding(.leading, 12)
                Text("润色中…").font(.system(size: 15)).foregroundStyle(.secondary)
                Spacer()
                close
            case .ready(_, let result):
                Text(result)
                    .font(.system(size: 17))
                    .foregroundStyle(Color.accentColor)
                    .lineLimit(1)
                    .truncationMode(.head)
                    .padding(.leading, 12)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
                    .onKeyboardTap { model.applyRewrite() }
                close
            case .failed:
                Text("润色没成功，检查网络后再试").font(.system(size: 15)).foregroundStyle(.secondary)
                    .padding(.leading, 12)
                Spacer()
                close
            }
        }
    }

    private var close: some View {
        Image(systemName: "xmark")
            .font(.system(size: 15, weight: .medium))
            .foregroundStyle(.secondary)
            .frame(width: 44, height: KeyStyle.candidateBarHeight)
            .onKeyboardPress { model.dismissRewrite() }
            .accessibilityLabel("放弃润色")
    }
}
