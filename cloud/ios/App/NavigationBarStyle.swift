// 导航栏标题（普通与大标题）用 MiSans：SwiftUI 的 .navigationTitle 由 UIKit 画，环境字体管不到，只能经 UINavigationBarAppearance 全局设一次。
// 背景照系统默认：滚动时有底，滚到顶透明。

import UIKit

enum NavigationBarStyle {
    @MainActor
    static func apply() {
        let title: [NSAttributedString.Key: Any] = [.font: AppFont.uiFont(size: 17, weight: .semibold)]
        let largeTitle: [NSAttributedString.Key: Any] = [.font: AppFont.uiFont(size: 34, weight: .bold)]
        let standard = UINavigationBarAppearance()
        standard.configureWithDefaultBackground()
        let scrollEdge = UINavigationBarAppearance()
        scrollEdge.configureWithTransparentBackground()
        for appearance in [standard, scrollEdge] {
            appearance.titleTextAttributes = title
            appearance.largeTitleTextAttributes = largeTitle
        }
        let bar = UINavigationBar.appearance()
        bar.standardAppearance = standard
        bar.compactAppearance = standard
        bar.scrollEdgeAppearance = scrollEdge
        bar.compactScrollEdgeAppearance = scrollEdge
    }
}
