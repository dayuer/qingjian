// 引导第一步「介绍」（04 的 1a）：图标与「素笺」、插图、衬线大标题「做一个记得对方的人」、说明、底部「开始」。
// 插图位置与尺寸照设计稿固定（通栏、高 260、圆角 14）：资源目录里有 onboarding-intro 就显示它（铺满裁切），
// 没有时画设计稿 .ph 的斜纹占位。插图导出后只加这个图片资源，不改布局。

import SwiftUI
import UIKit

struct IntroStep: View {
    static let illustration = "onboarding-intro"

    static let illustrationHeight: CGFloat = 260

    static let title = "做一个\n记得对方的人"

    static let detail = "打字的时候，它会想起她说过的事、你们约好的事。不用离开聊天。"

    static let start = "开始"

    /// 占位块里的说明（设计稿 .ph 的字），插图到位后不再显示。
    static let placeholderNote = "插图：键盘上方浮现一行「她周三考科目二」的瞬间"

    let onStart: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 12) {
                Image("brand-icon")
                    .resizable()
                    .frame(width: 44, height: 44)
                    .clipShape(RoundedRectangle(cornerRadius: 10))
                    .overlay(RoundedRectangle(cornerRadius: 10).strokeBorder(Color(.separator).opacity(0.5)))
                    .accessibilityHidden(true)
                Text("素笺")
                    .font(.system(size: 22, weight: .semibold, design: .serif))
                    .tracking(3)
            }
            .padding(.top, 28)
            illustration
                .frame(maxWidth: .infinity)
                .frame(height: Self.illustrationHeight)
                .clipShape(RoundedRectangle(cornerRadius: 14))
                .padding(.top, 28)
            VStack(alignment: .leading, spacing: 12) {
                Text(Self.title)
                    .font(.system(size: 32, weight: .semibold, design: .serif))
                    .lineSpacing(8)
                Text(Self.detail)
                    .font(.system(size: 15))
                    .lineSpacing(8)
                    .foregroundStyle(Theme.ink2)
            }
            .padding(.top, 36)
            Spacer(minLength: 24)
            Button(Self.start, action: onStart)
                .buttonStyle(OnboardingButtonStyle())
        }
        .padding(.horizontal, 28)
        .padding(.bottom, 12)
    }

    @ViewBuilder
    private var illustration: some View {
        if UIImage(named: Self.illustration) != nil {
            Image(Self.illustration).resizable().scaledToFill()
        } else {
            placeholder
        }
    }

    /// 设计稿 .ph：paper-2 底上每 9pt 一道 135° 的 paper-3 细线，中间等宽小字。
    private var placeholder: some View {
        Canvas { context, size in
            var path = Path()
            var x = -size.height
            while x < size.width {
                path.move(to: CGPoint(x: x, y: size.height))
                path.addLine(to: CGPoint(x: x + size.height, y: 0))
                x += 9 * 2.squareRoot()
            }
            context.stroke(path, with: .color(Color(.systemGray4)), lineWidth: 1)
        }
        .background(Color(.systemGray6))
        .overlay {
            Text(Self.placeholderNote)
                .font(.system(size: 11, design: .monospaced))
                .foregroundStyle(Theme.ink3)
                .multilineTextAlignment(.center)
                .padding(8)
        }
        .accessibilityHidden(true)
    }
}
