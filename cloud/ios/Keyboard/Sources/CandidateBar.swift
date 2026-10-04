// 键盘顶部的候选栏：横向滚动的候选，首选灰底；右端 ⌄ 展开全部候选。拼音不在这里，写在宿主的 marked text 里。
// 首选加粗，恋爱、日常选了人时再用灰绿字（设计稿 .k-cands .first）。没在组字时换成 IdleBar（改写、跨设备剪贴板、隐私输入的锁）。

import SwiftUI

struct CandidateBar: View {
    let model: KeyboardModel

    /// ⌄ 的宽度；它的触摸由 KeyTouchView 收（SwiftUI 手势在这里常吞短点击），这里只画。
    static let chevronWidth: CGFloat = 48

    var body: some View {
        // 对象卡与选择面板打开时这一行是它们的工具栏（牌子加收起），组字中打开的也一样（设计稿 1b）
        if model.composing && model.panel != .contactCard && model.panel != .scope {
            candidates
        } else {
            IdleBar(model: model)
        }
    }

    private var candidates: some View {
        ScrollViewReader { proxy in
            ScrollView(.horizontal, showsIndicators: false) {
                LazyHStack(spacing: 2) {
                    ForEach(Array(model.candidates.enumerated()), id: \.offset) { index, item in
                        CandidateCell(
                            text: item.text, highlighted: index == 0, cloud: item.cloud,
                            accent: index == 0 && model.accentFirstCandidate
                        ) {
                            model.selectCandidate(index)
                        }
                        .id(index)
                    }
                }
                .padding(.horizontal, 4)
            }
            // 滚出可视区的候选仍会抢点击，裁掉；右边给 ⌄ 让出位置
            .clipped()
            .contentShape(Rectangle())
            .padding(.trailing, model.composing ? Self.chevronWidth : 0)
            // 拼音变了候选从头显示
            .onChange(of: model.preedit) {
                proxy.scrollTo(0, anchor: .leading)
            }
        }
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
