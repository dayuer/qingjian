// 按键反馈：系统键盘音（跟随系统声音设置），开了完全访问时再加一下轻震。
// 第三方键盘没有完全访问时 UIFeedbackGenerator 调了也不震，这是 iOS 的限制。

import UIKit

@MainActor
final class KeyFeedback {
    private let haptics = UIImpactFeedbackGenerator(style: .light)

    /// 由控制器按 hasFullAccess 更新；用户可能在键盘出现期间去设置里改。
    var hapticsEnabled = false {
        didSet { if hapticsEnabled { haptics.prepare() } }
    }

    func keyDown() {
        // 只有 inputView 遵守 UIInputViewAudioFeedback 且可见时才会响，见 KeyboardInputView
        UIDevice.current.playInputClick()
        guard hapticsEnabled else { return }
        haptics.impactOccurred()
        haptics.prepare()
    }
}
