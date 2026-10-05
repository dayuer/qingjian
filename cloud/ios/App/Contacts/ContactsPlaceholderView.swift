// 「通讯录」Tab 的占位页，T4 做通讯录时换掉。

import SwiftUI

struct ContactsPlaceholderView: View {
    var body: some View {
        NavigationStack {
            VStack(spacing: 0) {
                // 与另外两页共用同一个标题件，高度才对得齐
                PageHeader(title: "通讯录")
                Text("通讯录正在做")
                    .foregroundStyle(.secondary)
                    .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
            .toolbar(.hidden, for: .navigationBar)
        }
    }
}
