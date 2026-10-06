// 主 App 入口：根视图是 RootView。键盘本身在扩展里，不依赖主 App 运行。

import SwiftUI
import QingjianBridge

@main
struct QingjianApp: App {
    init() {
        TabBarStyle.apply()
        NavigationBarStyle.apply()
        SegmentedControlStyle.apply()
    }

    @Environment(\.scenePhase) private var scenePhase

    var body: some Scene {
        WindowGroup {
            RootView()
        }
        .onChange(of: scenePhase) { _, phase in
            // 回前台踢一脚后台上传（素材与输入日志）；节流在上传线程里
            if phase == .active, let dir = SharedStore.directory {
                dir.path.withCString { qj_upload_kick($0) }
            }
        }
    }
}
