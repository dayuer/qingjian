// 键盘与 App 共用的颜色角色：灰绿三档只给对象相关的显示，其余用中性的 ink / ink2。各元素用哪一档见 ColorUsage。

import SwiftUI

enum ColorRole: Equatable {
    /// 灰绿强调色的文字档 accentInk（对象相关的文字、描边）。
    case accent

    /// 灰绿的实底 accent（App 开关打开时的底色）。
    case accentFill

    /// 灰绿的浅底 accentSoft（提示行、提醒卡的底色）。
    case accentSoft

    /// 中性的浅底（代替灰绿浅底与实底）。
    case neutralSoft

    /// 系统 label。
    case ink

    /// 系统 secondaryLabel。
    case ink2

    var color: Color {
        switch self {
        case .accent: Theme.accentInk.color
        case .accentFill: Theme.accent.color
        case .accentSoft: Theme.accentSoft.color
        case .neutralSoft: Color.secondary.opacity(0.12)
        case .ink: Theme.ink
        case .ink2: Theme.ink2
        }
    }

    /// 灰绿三档之一。
    var isAccent: Bool { self == .accent || self == .accentFill || self == .accentSoft }
}
