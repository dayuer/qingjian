// 「懒得自己写？」点进来的说明页：素笺云服务的自动记忆还没上线，这里只说明它会是什么样、现在的记忆在哪。

import SwiftUI

struct CloudIntroView: View {
    var body: some View {
        List {
            Section("现在") {
                Text("「键盘记住的事」全部是你自己写的，只存在这台手机上，不联网、不用登录。")
            }
            Section("以后") {
                Text("开通素笺云服务后，键盘会在你选的场景里，把你发出的话去掉手机号、地址这类信息后上传，每天整理成记忆卡，等你确认了才生效。")
                Text("不开就永远不会上传；开了也随时可以停。")
            }
        }
        .navigationTitle("素笺云服务")
        .navigationBarTitleDisplayMode(.inline)
    }
}
