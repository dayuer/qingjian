// 「记录中」暂停之后的提示条（设计稿 05 的 2b）：占提示行那一行，说清暂停了什么、1 小时后自动恢复；
// 定时暂停那一句右边给「一直暂停」。底与记一笔的 toast 同色（KeyStyle.keyFill），不是提示行的灰绿底——
// 这一条说的是「没在记」，用代表人的灰绿会让人以为还在记。

import SwiftUI

struct PausedBanner: View {
    let text: String

    /// 定时暂停那一句才带「一直暂停」；一直暂停与恢复那两句不带。
    let showsPauseForever: Bool

    let onPauseForever: () -> Void

    var body: some View {
        HStack(spacing: 8) {
            Circle()
                .fill(Theme.ink3)
                .frame(width: 6, height: 6)
                .padding(.leading, 12)
            Text(text)
                .font(.system(size: 12.5))
                .foregroundStyle(Theme.ink2)
                .lineLimit(1)
                .frame(maxWidth: .infinity, alignment: .leading)
            if showsPauseForever {
                Text(RecordingDisplay.pauseForever)
                    .font(.system(size: 12.5))
                    .foregroundStyle(ColorUsage.recordingBadge.role.color)
                    .padding(.horizontal, 12)
                    .frame(maxHeight: .infinity)
                    .onKeyboardPress(onPauseForever)
                    .accessibilityAddTraits(.isButton)
            }
        }
        .frame(height: KeyStyle.hintRowHeight)
        .background(KeyStyle.keyFill)
        .overlay(alignment: .bottom) { Rectangle().fill(Hairline.line).frame(height: 1) }
        .accessibilityElement(children: .combine)
        .accessibilityLabel(text)
    }
}
