// 引导底部的大按钮（设计稿 .btn.lg）：主按钮 ink 实底、paper 字、高 50 胶囊；次按钮（.btn.ghost）只有 ink-2 字、高 36。

import SwiftUI

struct OnboardingButtonStyle: ButtonStyle {
    /// true 为主按钮（.btn.pri），false 为次按钮（.btn.ghost）。
    var primary = true

    func makeBody(configuration: Configuration) -> some View {
        configuration.label
            .font(.system(size: 16, weight: .medium))
            .foregroundStyle(primary ? Color(.systemBackground) : Theme.ink2)
            .frame(maxWidth: .infinity, minHeight: primary ? 50 : 36)
            .background(primary ? Theme.ink : .clear, in: Capsule())
            .contentShape(Capsule())
            .opacity(configuration.isPressed ? 0.7 : 1)
    }
}
