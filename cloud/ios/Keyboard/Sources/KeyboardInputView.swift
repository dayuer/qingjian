// 键盘扩展的 inputView：声明要系统键盘音，UIDevice.playInputClick() 才会响；
// allowsSelfSizing 让系统一开始就按我们的 Auto Layout 高度给，不先沿用上一个键盘的高度再改（那样微信这类宿主按旧高度布局，输入栏被盖住）。

import UIKit

final class KeyboardInputView: UIInputView, UIInputViewAudioFeedback {
    var enableInputClicksWhenVisible: Bool { true }

    override init(frame: CGRect, inputViewStyle: UIInputView.Style) {
        super.init(frame: frame, inputViewStyle: inputViewStyle)
        allowsSelfSizing = true
    }

    required init?(coder: NSCoder) {
        super.init(coder: coder)
        allowsSelfSizing = true
    }
}
