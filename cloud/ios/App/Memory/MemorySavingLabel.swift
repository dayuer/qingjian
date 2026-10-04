// 「正在保存」：转圈加一句话，编辑页的完成按钮与加人页的大按钮在后台写入时换成它；颜色由外面定。

import SwiftUI

struct MemorySavingLabel: View {
    var body: some View {
        HStack(spacing: 6) {
            ProgressView()
            Text(MemoryStore.Wording.saving)
        }
        .accessibilityElement(children: .combine)
    }
}
