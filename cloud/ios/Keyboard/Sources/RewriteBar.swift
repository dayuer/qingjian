// 改写进行中的候选栏（等结果 / 结果出来点一下替换原文 / 失败），✕ 放弃；空闲时的入口在 IdleBar。

import SwiftUI

struct RewriteBar: View {
    let model: KeyboardModel

    var body: some View {
        HStack(spacing: 8) {
            switch model.rewrite {
            case .idle:
                EmptyView()
            case .pending:
                ProgressView().padding(.leading, 12)
                Text("改写中…").font(.system(size: 15)).foregroundStyle(.secondary)
                Spacer()
                close
            case .ready(_, let result):
                Text(result)
                    .font(.system(size: 17))
                    .foregroundStyle(Theme.ink)
                    .lineLimit(1)
                    .truncationMode(.head)
                    .padding(.leading, 12)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
                    .onKeyboardTap { model.applyRewrite() }
                close
            case .failed:
                Text("改写没成功，检查网络后再试").font(.system(size: 15)).foregroundStyle(.secondary)
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
            .accessibilityLabel("放弃改写")
    }
}
