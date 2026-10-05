// 键盘顶部的候选栏：组字时候选那一行由控制器挂的 UIKit 视图 CandidateBarView 画与收触摸
// （SwiftUI 的 ScrollView 在键盘扩展里收不到滑动），这里只画右端的 ⌄。
// 拼音不在这里，写在宿主的 marked text 里。没在组字时换成 IdleBar（改写、跨设备剪贴板、隐私输入的锁）。

import SwiftUI

struct CandidateBar: View {
    let model: KeyboardModel

    /// 这些面板打开时，候选栏那一行换成它们自己的工具栏（牌子 + 收起），没有 ⌄。
    static func panelTakesTheBar(_ panel: KeyboardPanel) -> Bool {
        panel == .contactCard || panel == .scope || panel == .draft || panel == .conflict
    }

    /// ⌄ 的宽度；它的触摸由 KeyTouchView 收（SwiftUI 手势在这里常吞短点击），这里只画。
    static let chevronWidth: CGFloat = 48

    var body: some View {
        // 对象卡与选择面板打开时这一行是它们的工具栏（牌子加收起），组字中打开的也一样（设计稿 1b）
        if model.composing && !Self.panelTakesTheBar(model.panel) {
            candidates
        } else {
            IdleBar(model: model)
        }
    }

    private var candidates: some View {
        // 只留位置：候选本身由 CandidateBarView 画在这一条上（控制器按 hintInset 摆它的 frame）
        Color.clear
            .frame(maxWidth: .infinity)
            .frame(height: KeyStyle.candidateBarHeight)
            .overlay(alignment: .trailing) {
                if model.composing { chevron }
            }
    }

    private var chevron: some View {
        HStack(spacing: 0) {
            Divider().frame(height: 26)
            Image(systemName: model.panel == .candidates ? "chevron.up" : "chevron.down")
                .font(.system(size: 18, weight: .medium))
                .foregroundStyle(.primary)
                .frame(width: Self.chevronWidth - 1, height: KeyStyle.candidateBarHeight)
        }
        .zIndex(1)
        .accessibilityElement(children: .ignore)
        .accessibilityAddTraits(.isButton)
        .accessibilityLabel(model.panel == .candidates ? "收起候选" : "展开候选")
    }
}
