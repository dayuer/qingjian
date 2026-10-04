// 主 App 入口：根视图是 RootView。键盘本身在扩展里，不依赖主 App 运行。

import SwiftUI

@main
struct QingjianApp: App {
    init() {
        TabBarStyle.apply()
    }

    var body: some Scene {
        WindowGroup {
            RootView()
        }
    }
}
