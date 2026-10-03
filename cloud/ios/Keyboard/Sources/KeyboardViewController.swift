// 键盘扩展入口：打开引擎、挂上 SwiftUI 键盘，把输出接到 textDocumentProxy（拼音走 marked text）。

import SwiftUI
import UIKit

final class KeyboardViewController: UIInputViewController, TextOutput {
    private var model: KeyboardModel!

    private var hosting: UIHostingController<KeyboardView>?

    /// 宿主里现在有没有我们写的 marked text；proxy 没有接口能问，自己记。
    private var hasMarkedText = false

    private let feedback = KeyFeedback()

    override func loadView() {
        super.loadView()
        inputView = KeyboardInputView(frame: .zero, inputViewStyle: .keyboard)
    }

    override func viewDidLoad() {
        super.viewDidLoad()
        model = KeyboardModel(engine: Self.openEngine())
        model.output = self
        model.onKeyDown = { [feedback] in feedback.keyDown() }
        mountKeyboard()
    }

    override func viewWillAppear(_ animated: Bool) {
        super.viewWillAppear(animated)
        // 完全访问只用来震动；用户随时可能去设置里开关，每次出现时重读
        feedback.hapticsEnabled = hasFullAccess
    }

    override func viewWillLayoutSubviews() {
        super.viewWillLayoutSubviews()
        // needsInputModeSwitchKey 要等视图出现后才准
        if hosting?.rootView.showsGlobe != needsInputModeSwitchKey {
            hosting?.rootView = KeyboardView(model: model, showsGlobe: needsInputModeSwitchKey)
        }
    }

    override func viewWillDisappear(_ animated: Bool) {
        super.viewWillDisappear(animated)
        model.dismiss()
    }

    func setMarked(_ text: String) {
        if text.isEmpty {
            guard hasMarkedText else { return }
            textDocumentProxy.setMarkedText("", selectedRange: NSRange(location: 0, length: 0))
            textDocumentProxy.unmarkText()
            hasMarkedText = false
            return
        }
        textDocumentProxy.setMarkedText(text, selectedRange: NSRange(location: text.utf16.count, length: 0))
        hasMarkedText = true
    }

    /// insertText 会替换掉宿主里的 marked text。不用「setMarkedText 再 unmarkText」：
    /// 紧接着写下一段 marked text（候选只吃掉一部分拼音时）会把刚上屏的字也换掉。
    func commit(_ text: String) {
        textDocumentProxy.insertText(text)
        hasMarkedText = false
    }

    func deleteBackward() {
        textDocumentProxy.deleteBackward()
    }

    func switchToNextKeyboard() {
        advanceToNextInputMode()
    }

    private func mountKeyboard() {
        let hosting = UIHostingController(
            rootView: KeyboardView(model: model, showsGlobe: needsInputModeSwitchKey))
        hosting.view.backgroundColor = .clear
        hosting.view.translatesAutoresizingMaskIntoConstraints = false
        addChild(hosting)
        view.addSubview(hosting.view)
        let height = hosting.view.heightAnchor.constraint(
            equalToConstant: KeyStyle.candidateBarHeight + KeyboardView.keyAreaHeight)
        // 系统旋转或切换时会临时塞一个冲突的高度约束，留一档优先级让它赢
        height.priority = .defaultHigh
        NSLayoutConstraint.activate([
            hosting.view.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            hosting.view.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            hosting.view.topAnchor.constraint(equalTo: view.topAnchor),
            hosting.view.bottomAnchor.constraint(equalTo: view.bottomAnchor),
            height,
        ])
        hosting.didMove(toParent: self)
        self.hosting = hosting
    }

    /// 产品数据随扩展打包在 Data/ 下；学习数据放在扩展自己的容器里（没有完全访问权限也能写）。
    private static func openEngine() -> Engine? {
        guard let data = Bundle.main.url(forResource: "Data", withExtension: nil) else {
            return nil
        }
        let support = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask)
            .first?.appendingPathComponent("Qingjian", isDirectory: true)
        if let support {
            try? FileManager.default.createDirectory(at: support, withIntermediateDirectories: true)
        }
        return Engine(dataDirectory: data, userDirectory: support)
    }
}
