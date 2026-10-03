// 键盘扩展的 inputView：声明要系统键盘音，UIDevice.playInputClick() 才会响。

import UIKit

final class KeyboardInputView: UIInputView, UIInputViewAudioFeedback {
    var enableInputClicksWhenVisible: Bool { true }

}
