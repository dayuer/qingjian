// 引导第三步「免费版与云服务」（05 的 2e）：两张方案卡，免费那张描灰绿边、标「现在就是」；主按钮「先用免费版」，
// 次按钮「了解云服务」先推到现有的说明页 CloudIntroView（T9 换成开通流程）。不在引导里推付费，界面不出现账号一类的字（UI 清单约束 5）。

import SwiftUI

struct PlanStep: View {
    static let title = "要不要让键盘自己记？"

    static let freeTitle = "免费"

    static let freeBadge = "现在就是"

    static let freeItems = ["完整的输入法", "自己写记忆卡，打字时提示", "什么都不上传"]

    static let cloudTitle = "素笺云服务"

    /// 价格是设计稿的占位，订阅走苹果内购，定价后再改。
    static let cloudPrice = "¥ — / 月"

    static let cloudItems = ["你开启的场景里，键盘自己记、每天整理", "每周一张「这周记住了什么」，你确认", "选中文字一键改写", "换手机记忆还在"]

    static let useFree = "先用免费版"

    static let learnCloud = "了解云服务"

    /// 这一步界面上的全部文字（测试用来查有没有不该出现的词）。
    static var allTexts: [String] {
        [title, freeTitle, freeBadge, cloudTitle, cloudPrice, useFree, learnCloud] + freeItems + cloudItems
    }

    let onNext: () -> Void

    let onLearnCloud: () -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            OnboardingProgressBar(step: .plan).padding(.horizontal, 4).padding(.top, 16)
            Text(Self.title)
                .font(AppFont.font(size: 26, weight: .semibold))
                .padding(.horizontal, 4)
                .padding(.top, 18)
            plan(Self.freeTitle, items: Self.freeItems, current: true) {
                Text(Self.freeBadge)
                    .font(AppFont.font(size: 11, weight: .medium))
                    .foregroundStyle(ColorUsage.onboardingCurrentPlan.role.color)
                    .padding(.horizontal, 8)
                    .frame(height: 20)
                    .background(ColorUsage.onboardingPlanBadge.role.color, in: Capsule())
            }
            plan(Self.cloudTitle, items: Self.cloudItems, current: false) {
                Text(Self.cloudPrice).font(AppFont.font(size: 15, weight: .semibold))
            }
            Spacer(minLength: 0)
            VStack(spacing: 0) {
                Button(Self.useFree, action: onNext).buttonStyle(OnboardingButtonStyle())
                Button(Self.learnCloud, action: onLearnCloud)
                    .buttonStyle(OnboardingButtonStyle(primary: false))
                    .padding(.top, 8)
            }
        }
        .padding(.horizontal, 20)
        .padding(.bottom, 12)
    }

    /// 设计稿 .plan：白底、18 圆角、细描边、18pt 内边距；当前方案（.plan.on）换 2pt 灰绿描边。
    private func plan(_ title: String, items: [String], current: Bool, @ViewBuilder trailing: () -> some View) -> some View {
        VStack(alignment: .leading, spacing: 10) {
            HStack {
                Text(title).font(AppFont.font(size: 18, weight: .semibold))
                Spacer()
                trailing()
            }
            VStack(alignment: .leading, spacing: 0) {
                ForEach(items, id: \.self) { item in
                    HStack(alignment: .firstTextBaseline, spacing: 8) {
                        Text("•")
                        Text(item)
                    }
                    .font(AppFont.font(size: 13.5))
                    .lineSpacing(6)
                    .padding(.vertical, 2.5)
                    .foregroundStyle(Theme.ink2)
                }
            }
        }
        .padding(18)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(Color(.secondarySystemGroupedBackground), in: RoundedRectangle(cornerRadius: 18))
        .overlay {
            RoundedRectangle(cornerRadius: 18).strokeBorder(
                current ? ColorUsage.onboardingCurrentPlan.role.color : Color(.separator).opacity(0.5),
                lineWidth: current ? 2 : 1)
        }
    }
}
