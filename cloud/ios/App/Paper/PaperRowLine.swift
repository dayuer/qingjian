// 设计稿 .row / .mem 的下沿：分组里行与行之间 1pt 的淡灰线，通栏（不缩进）。

import SwiftUI

struct PaperRowLine: View {
    var body: some View {
        Rectangle().fill(Hairline.row).frame(height: 1)
    }
}
