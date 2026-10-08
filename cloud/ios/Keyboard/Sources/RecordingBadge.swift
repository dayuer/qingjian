// 工具栏牌子后面的「记录中 / 已暂停」（设计稿 05 的 2a / 2b）：记录中是灰绿实心点，暂停了是空心圈。
// 点一下：记录中 → 暂停 1 小时；已暂停 → 立刻恢复。整块（高 30、左右 6）都是点按区域。

import SwiftUI

struct RecordingBadge: View {
    let badge: RecordingDisplay.Badge

    let action: () -> Void

    var body: some View {
        HStack(spacing: 4) {
            dot
            Text(badge.title)
                .font(.system(size: 12))
                .foregroundStyle(color)
        }
        .padding(.horizontal, 6)
        .frame(height: 30)
        .contentShape(Rectangle())
        .onKeyboardPress(action)
        .accessibilityAddTraits(.isButton)
        .accessibilityLabel(badge == .recording ? "正在记录，点一下暂停" : "已暂停记录，点一下恢复")
    }

    private var color: Color {
        switch badge {
        case .recording: ColorUsage.recordingBadge.role.color
        case .paused: ColorUsage.pausedBadge.role.color
        }
    }

    @ViewBuilder
    private var dot: some View {
        switch badge {
        case .recording:
            Circle().fill(color).frame(width: 6, height: 6)
        case .paused:
            Circle().strokeBorder(color, lineWidth: 1.2).frame(width: 6, height: 6)
        }
    }
}
