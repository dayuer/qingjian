// 引导顶部的进度条（设计稿 theme.css 的 .progress）：四格等宽、3pt 高、间距 5pt，亮的格子灰绿实底，没到的 paper-3。

import SwiftUI

struct OnboardingProgressBar: View {
    let step: OnboardingStep

    var body: some View {
        HStack(spacing: 5) {
            ForEach(Array(step.progressCells.enumerated()), id: \.offset) { _, on in
                RoundedRectangle(cornerRadius: 2)
                    .fill(on ? ColorUsage.onboardingProgress.role.color : Color(.systemGray5))
                    .frame(height: 3)
            }
        }
        .accessibilityElement()
        .accessibilityLabel("第 \(step.number) 步，共 \(OnboardingStep.allCases.count) 步")
    }
}
