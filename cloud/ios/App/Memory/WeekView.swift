// 「本周」：开着日子提醒的人 7 天内（今天到 6 天后）的日子与约定，按日期排。从「记得」首页顶上的入口推进来，T5 并进首页后删掉。

import SwiftUI

struct WeekView: View {
    let store: MemoryStore

    var body: some View {
        List {
            if let error = store.loadError {
                MemoryFailureBanner(text: error) { Task { await store.reload() } }
            }
            let items = store.upcoming(within: 6)
            if items.isEmpty && store.loadError == nil {
                Text("这 7 天没有记下的日子和约定").foregroundStyle(.secondary)
            }
            ForEach(items) { item in
                HStack {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(item.card.text)
                        Text("\(item.contact.name) · \(item.card.kind.title)")
                            .font(AppFont.caption)
                            .foregroundStyle(.secondary)
                    }
                    Spacer()
                    Text(item.dayLabel).font(AppFont.caption).foregroundStyle(.secondary)
                }
            }
        }
        .navigationTitle("本周")
        .refreshable { await store.reload() }
    }
}
