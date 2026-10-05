// 通讯录右侧的字母索引（设计稿 02 的 2b）：A–Z 与「#」竖贴在右边缘，点或按住上下拖跳到那一节。
// 每个字母固定 15pt 一行（设计稿 `font:600 11px/15px`），所以不用量高度就能把手指的 y 换成第几个字母。

import SwiftUI

struct LetterIndex: View {
    let letters: [String]

    let onPick: (String) -> Void

    /// 一行一个字母的高度。
    static let rowHeight: CGFloat = 15

    var body: some View {
        VStack(spacing: 0) {
            ForEach(letters, id: \.self) { letter in
                Text(letter)
                    .font(AppFont.font(size: 11, weight: .semibold))
                    .foregroundStyle(letter == ContactIndex.other ? Theme.ink3 : Theme.ink2)
                    .frame(width: 16, height: Self.rowHeight)
            }
        }
        .contentShape(Rectangle())
        // 按下就跳（minimumDistance 0），滑动时一路跟手；同时点一下也算
        .gesture(
            DragGesture(minimumDistance: 0)
                .onChanged { value in
                    let index = Int(value.location.y / Self.rowHeight)
                    guard letters.indices.contains(index) else { return }
                    onPick(letters[index])
                })
        .accessibilityHidden(true)
    }
}
