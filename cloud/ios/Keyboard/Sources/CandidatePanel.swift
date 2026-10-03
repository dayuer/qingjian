// 点 ⌄ 展开的全部候选：占满键区，按候选长短自动换行，纵向滚动。

import SwiftUI

struct CandidatePanel: View {
    let model: KeyboardModel

    var body: some View {
        ScrollView {
            LazyVGrid(columns: [GridItem(.adaptive(minimum: 56), spacing: 4)], spacing: 6) {
                ForEach(Array(model.candidates.enumerated()), id: \.offset) { index, item in
                    CandidateCell(text: item.text, highlighted: index == 0, cloud: item.cloud) {
                        model.selectCandidate(index)
                    }
                }
            }
            .padding(.horizontal, KeyStyle.sideMargin)
            .padding(.vertical, 6)
        }
    }
}
