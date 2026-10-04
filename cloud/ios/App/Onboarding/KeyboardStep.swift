// 引导第二步「开启键盘」（05 的 2d）：添加键盘的路径、「这就能用了」与完全访问两行说明、底部「去设置添加」。
// 完全访问这一行与键盘面板、「我」页同一段话（ScopeDisplay，T2）：记忆要开完全访问，但打字不用，所以这一步可以跳过。
// 「去设置添加」跳到素笺在系统设置里的页面；从设置回来自动进下一步。

import SwiftUI
import UIKit

struct KeyboardStep: View {
    static let title = "先打开键盘"

    static let path = "设置 → 通用 → 键盘 → 添加新键盘 → "

    static let readyTitle = "这就能用了"

    /// 设计稿这里还写了「自己写记忆卡、打字时提示」，但读卡片要完全访问（UI 清单约束 3），这里只写不开也能用的。
    static let readyDetail = "打字、记住你的用词习惯，全在手机上，不联网，免费。"

    static let fullAccessTitle = "「允许完全访问」可以先不开"

    static let fullAccessText = ScopeDisplay.needsFullAccessText

    static let fullAccessPath = ScopeDisplay.fullAccessPath

    static let openSettings = "去设置添加"

    static let skip = "先跳过"

    let onNext: () -> Void

    @Environment(\.scenePhase) private var scenePhase

    @State private var wentToSettings = false

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            OnboardingProgressBar(step: .keyboard).padding(.top, 16)
            Text(Self.title)
                .font(.system(size: 26, weight: .semibold, design: .serif))
                .padding(.top, 32)
            HStack(alignment: .top, spacing: 12) {
                Text("✓")
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundStyle(Theme.ink)
                    .frame(width: 24, height: 24)
                    .background(ColorUsage.onboardingStepDone.role.color, in: Circle())
                (Text(Self.path).foregroundStyle(Theme.ink2)
                    + Text("素笺").fontWeight(.medium).foregroundStyle(Theme.ink))
                    .font(.system(size: 14.5))
                    .lineSpacing(4)
            }
            .padding(.top, 24)
            VStack(alignment: .leading, spacing: 0) {
                row(Self.readyTitle) { detail(Self.readyDetail) }
                Divider()
                row(Self.fullAccessTitle) {
                    detail(Self.fullAccessText)
                    Text(Self.fullAccessPath)
                        .font(.system(size: 12.5))
                        .foregroundStyle(Theme.ink3)
                        .padding(.top, 4)
                }
            }
            .background(Color(.secondarySystemGroupedBackground), in: RoundedRectangle(cornerRadius: 14))
            .overlay(RoundedRectangle(cornerRadius: 14).strokeBorder(Color(.separator).opacity(0.5)))
            .padding(.top, 28)
            Spacer(minLength: 24)
            Button(Self.openSettings) {
                guard let url = URL(string: UIApplication.openSettingsURLString) else { return }
                wentToSettings = true
                UIApplication.shared.open(url)
            }
            .buttonStyle(OnboardingButtonStyle())
            Button(Self.skip, action: onNext)
                .buttonStyle(OnboardingButtonStyle(primary: false))
                .padding(.top, 6)
        }
        .padding(.horizontal, 24)
        .padding(.bottom, 12)
        .onChange(of: scenePhase) { _, phase in
            guard phase == .active, wentToSettings else { return }
            wentToSettings = false
            onNext()
        }
    }

    private func row(_ title: String, @ViewBuilder content: () -> some View) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(title).font(.system(size: 15, weight: .medium))
            content()
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, 14)
        .padding(.vertical, 12)
    }

    private func detail(_ text: String) -> some View {
        Text(text)
            .font(.system(size: 13.5))
            .lineSpacing(5)
            .foregroundStyle(Theme.ink3)
    }
}
