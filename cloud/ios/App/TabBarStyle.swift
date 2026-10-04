// 底部标签栏照设计稿 .tab：选中项 ink（系统 label）、未选中 ink-3（系统 tertiaryLabel），不用系统蓝。
// SwiftUI 的 TabView 没有分别设选中 / 未选中颜色的接口，只能经 UITabBarAppearance 全局设一次。

import UIKit

enum TabBarStyle {
    static let selected = UIColor.label

    static let normal = UIColor.tertiaryLabel

    static func apply() {
        let appearance = UITabBarAppearance()
        appearance.configureWithDefaultBackground()
        for layout in [appearance.stackedLayoutAppearance, appearance.inlineLayoutAppearance, appearance.compactInlineLayoutAppearance] {
            layout.selected.iconColor = selected
            layout.selected.titleTextAttributes = [.foregroundColor: selected]
            layout.normal.iconColor = normal
            layout.normal.titleTextAttributes = [.foregroundColor: normal]
        }
        UITabBar.appearance().standardAppearance = appearance
        UITabBar.appearance().scrollEdgeAppearance = appearance
    }
}
