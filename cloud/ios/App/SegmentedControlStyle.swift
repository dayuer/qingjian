// 分段选择（.pickerStyle(.segmented)，如对象设置里的「他 / 她 / TA」）用 MiSans：它由 UISegmentedControl 画，环境字体管不到，只能经 appearance 全局设一次。

import UIKit

enum SegmentedControlStyle {
    @MainActor
    static func apply() {
        let control = UISegmentedControl.appearance()
        control.setTitleTextAttributes([.font: AppFont.uiFont(size: 13, weight: .regular)], for: .normal)
        control.setTitleTextAttributes([.font: AppFont.uiFont(size: 13, weight: .semibold)], for: .selected)
    }
}
