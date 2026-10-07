// 设计稿 .lead：分组下面的说明段。缺省 13.5 ink-2 行高 1.6；1d 名字下那句是 12 ink-3（small）。左右 20。

import SwiftUI

struct PaperLead: View {
    let text: String

    var small = false

    var body: some View {
        Text(text)
            .font(AppFont.font(size: small ? 12 : 13.5))
            .lineSpacing(small ? 5 : 8)
            .foregroundStyle(small ? Theme.ink3 : Theme.ink2)
            .padding(.horizontal, 20)
            .frame(maxWidth: .infinity, alignment: .leading)
    }
}
