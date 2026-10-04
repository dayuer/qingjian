// 「通讯录」Tab 的占位页，T4 做通讯录时换掉。

import SwiftUI

struct ContactsPlaceholderView: View {
    var body: some View {
        NavigationStack {
            Text("通讯录正在做")
                .foregroundStyle(.secondary)
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                .navigationTitle("通讯录")
        }
    }
}
