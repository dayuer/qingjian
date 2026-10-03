// 主 App：告诉用户怎么启用键盘，给一个试打的输入框。键盘本身在扩展里，不依赖主 App 运行。

import SwiftUI

@main
struct QingjianApp: App {
    var body: some Scene {
        WindowGroup {
            SetupView()
        }
    }
}
