// 整个键盘：候选栏 + 键区（或展开的候选 / 表情面板）。键宽按「一行 10 个字母键铺满」算。

import SwiftUI

struct KeyboardView: View {
    let model: KeyboardModel

    /// 系统要求自带切换键（没有键盘下方的地球键）时，底行 😀 的位置换成地球键。
    let showsGlobe: Bool

    /// 键区高度：四行键加行距与上下留白，展开面板也占这么高，切换时键盘不跳。
    static let keyAreaHeight = KeyStyle.keyHeight * 4 + KeyStyle.rowSpacing * 3 + 14

    var body: some View {
        VStack(spacing: 0) {
            CandidateBar(model: model)
            Group {
                switch model.panel {
                case .keys: keys
                case .candidates: CandidatePanel(model: model)
                case .emoji: EmojiPanel(model: model)
                }
            }
            .frame(height: Self.keyAreaHeight)
        }
    }

    private var keys: some View {
        GeometryReader { geometry in
            let unit = KeyStyle.unit(for: geometry.size.width)
            VStack(spacing: KeyStyle.rowSpacing) {
                ForEach(Array(model.layer.rows.enumerated()), id: \.offset) { _, row in
                    rowView(row, unit: unit)
                }
                bottomRow(unit: unit)
            }
            .padding(.horizontal, KeyStyle.sideMargin)
            .padding(.top, 8)
        }
    }

    @ViewBuilder
    private func rowView(_ row: KeyRow, unit: CGFloat) -> some View {
        HStack(spacing: KeyStyle.keySpacing) {
            if let leading = row.leading {
                key(leading, width: unit * KeyStyle.edgeKeyUnits)
                if !row.stretchesMiddle { Spacer(minLength: 0) }
            }
            ForEach(Array(row.middle.enumerated()), id: \.offset) { _, middle in
                if row.stretchesMiddle {
                    KeyButton(key: middle, model: model)
                        .frame(maxWidth: .infinity)
                        .frame(height: KeyStyle.keyHeight)
                } else {
                    key(middle, width: unit)
                }
            }
            if let trailing = row.trailing {
                if !row.stretchesMiddle { Spacer(minLength: 0) }
                key(trailing, width: unit * KeyStyle.edgeKeyUnits)
            }
        }
        .frame(maxWidth: .infinity)
    }

    /// 底行：切层、😀（或地球键）、空格吃掉剩余宽度、换行。
    private func bottomRow(unit: CGFloat) -> some View {
        HStack(spacing: KeyStyle.keySpacing) {
            key(.layer(model.layer == .letters ? .digits : .letters), width: unit * KeyStyle.bottomSideUnits)
            key(showsGlobe ? .globe : .emoji, width: unit * KeyStyle.bottomSideUnits)
            KeyButton(key: .space, model: model)
                .frame(maxWidth: .infinity)
                .frame(height: KeyStyle.keyHeight)
            key(.returnKey, width: unit * KeyStyle.returnUnits)
        }
    }

    private func key(_ key: Key, width: CGFloat) -> some View {
        KeyButton(key: key, model: model).frame(width: width, height: KeyStyle.keyHeight)
    }
}
