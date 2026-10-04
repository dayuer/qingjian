// 键盘上文字与描边的颜色角色：accent 只给对象相关的显示（牌子、提示行、对象卡、选择面板里的对象、记一笔条），其余用中性的 ink / ink2。

import SwiftUI

enum ColorRole: Equatable {
    /// 灰绿强调色（对象相关）。
    case accent

    /// 系统 label。
    case ink

    /// 系统 secondaryLabel。
    case ink2

    var color: Color {
        switch self {
        case .accent: Theme.accentInk.color
        case .ink: Theme.ink
        case .ink2: Theme.ink2
        }
    }
}
