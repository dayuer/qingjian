// 键盘扩展入口：打开引擎、挂上 SwiftUI 键盘，把输出接到 textDocumentProxy（拼音走 marked text）。

import SwiftUI
import UIKit

final class KeyboardViewController: UIInputViewController, TextOutput {
    private var model: KeyboardModel!

    private var hosting: UIHostingController<KeyboardView>?

    /// 宿主里现在有没有我们写的 marked text；proxy 没有接口能问，自己记。
    private var hasMarkedText = false

    private let feedback = KeyFeedback()

    /// 键区的触摸层（SwiftUI 只画键）；展开候选或表情面板时藏起来。
    private let touchView = KeyTouchView()

    /// 展开的候选面板（UIKit，见 `syncPanel`）。
    private let panelView = CandidatePanelView()

    /// 键盘可见期间每 0.25 秒一次：大模型候选、润色结果、别的设备的学习数据都靠它取回。
    private var pollTimer: Timer?

    /// 打开引擎时的完全访问与 cloud.toml 修改时间；出现时对不上就重开引擎。
    private var engineSignature = ""

    /// 钉在 inputView 上的键盘高度；进出「恋爱 · 某人」时加减一行提示行（见 `syncHintRow`）。
    private var heightConstraint: NSLayoutConstraint?

    /// 上一次看到的宿主输入框（`textDocumentProxy.documentIdentifier`）。
    private var lastDocument: UUID?

    override func loadView() {
        super.loadView()
        inputView = KeyboardInputView(frame: .zero, inputViewStyle: .keyboard)
    }

    override func viewDidLoad() {
        super.viewDidLoad()
        engineSignature = currentSignature
        model = KeyboardModel(engine: Self.openEngine(fullAccess: hasFullAccess))
        model.output = self
        model.onKeyDown = { [feedback] in feedback.keyDown() }
        mountKeyboard()
        mountTouchView()
        syncHintRow()
        setNeedsUpdateOfScreenEdgesDeferringSystemGestures()
    }

    override func viewDidLayoutSubviews() {
        super.viewDidLayoutSubviews()
        let keyArea = CGRect(
            x: 0, y: hintInset + KeyStyle.candidateBarHeight, width: view.bounds.width,
            height: KeyboardView.keyAreaHeight)
        touchView.frame = view.bounds
        touchView.keyArea = keyArea
        panelView.frame = keyArea
        syncTouchView()
        syncPanel()
    }

    /// 屏幕左右边缘的触摸会被系统边缘手势压住（a、l 慢半拍或丢）：要我们先处理。iOS 在视图切换时会重置，所以 viewWillAppear 再要一次。
    override var preferredScreenEdgesDeferringSystemGestures: UIRectEdge { [.left, .right] }

    override func viewWillAppear(_ animated: Bool) {
        super.viewWillAppear(animated)
        // 完全访问用来震动与连青简 Cloud；用户随时可能去设置里开关，每次出现时重读
        feedback.hapticsEnabled = hasFullAccess
        setNeedsUpdateOfScreenEdgesDeferringSystemGestures()
        if currentSignature != engineSignature {
            engineSignature = currentSignature
            // 先释放旧引擎再建新的，只是缩小新旧 DataSync 线程重叠的窗口（旧线程空闲或在等待时，会在新线程起来前收到停止信号），
            // 并没有消除：DataSync 的 Drop 只置 stop 不 join，旧线程若正好在一轮 HTTP 里，结束时仍可能把进度文件写回。
            // 根治要 Rust 侧落盘前检查 stop（已知限制）
            model.replaceEngine(nil)
            model.replaceEngine(Self.openEngine(fullAccess: hasFullAccess))
        }
        updatePrivacy()
        model.fullAccess = hasFullAccess
        model.appear()
        pollTimer?.invalidate()
        pollTimer = Timer.scheduledTimer(withTimeInterval: 0.25, repeats: true) { [weak self] _ in
            MainActor.assumeIsolated { self?.model.poll() }
        }
    }

    /// 边缘手势的第二道：从我们的视图一路到窗口，把系统识别器的「先压住触摸」关掉、边缘滑动识别器禁用。窗口要等出现后才有。
    override func viewDidAppear(_ animated: Bool) {
        super.viewDidAppear(animated)
        var current: UIView? = view
        while let node = current {
            for recognizer in node.gestureRecognizers ?? [] {
                recognizer.delaysTouchesBegan = false
                recognizer.delaysTouchesEnded = false
                if recognizer is UIScreenEdgePanGestureRecognizer { recognizer.isEnabled = false }
            }
            current = node.superview
        }
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
        pollTimer?.invalidate()
        pollTimer = nil
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

    func moveCursor(by offset: Int) {
        textDocumentProxy.adjustTextPosition(byCharacterOffset: offset)
    }

    /// 同一次弹出里焦点也会换输入框（填完用户名跳到验证码），每次都重判；换了输入框还要清掉最近上屏的字，
    /// 免得在 A 聊天里打的字在 B 里触发记忆提示。
    override func textDidChange(_ textInput: (any UITextInput)?) {
        super.textDidChange(textInput)
        updatePrivacy()
        let document = hostDocumentIdentifier
        if HostDocument.changed(from: lastDocument, to: document) {
            lastDocument = document
            model.hostChanged()
        }
    }

    /// 宿主输入框的标识。`textDocumentProxy.documentIdentifier` 声明为非可选 UUID，但键盘刚弹出、连上宿主之前系统返回 nil，
    /// 直接读会在 UUID 桥接处 EXC_BREAKPOINT 崩溃，所以走 KVC 取成可选值，别「简化」回去。
    private var hostDocumentIdentifier: UUID? {
        (textDocumentProxy as? NSObject)?.value(forKey: "documentIdentifier") as? UUID
    }

    var contextBefore: String { textDocumentProxy.documentContextBeforeInput ?? "" }

    var contextAfter: String { textDocumentProxy.documentContextAfterInput ?? "" }

    func switchToNextKeyboard() {
        advanceToNextInputMode()
    }

    var pasteboardChangeCount: Int { UIPasteboard.general.changeCount }

    var pasteboardHasText: Bool { UIPasteboard.general.hasStrings }

    func readPasteboard() -> String? { UIPasteboard.general.string }

    /// 密码框（secureTextEntry）系统根本不给第三方键盘；这里挡的是验证码、新密码、信用卡号这类照样用我们键盘的字段。
    private func updatePrivacy() {
        let proxy = textDocumentProxy
        let secure = proxy.isSecureTextEntry ?? false
        let sensitive = proxy.textContentType.map(Self.sensitiveContentTypes.contains) ?? false
        model.setPrivateField(secure || sensitive)
    }

    private static let sensitiveContentTypes: Set<UITextContentType> = [
        .password, .newPassword, .oneTimeCode, .creditCardNumber,
        .creditCardSecurityCode, .creditCardExpiration, .creditCardExpirationMonth,
        .creditCardExpirationYear,
    ]

    private func mountKeyboard() {
        let hosting = UIHostingController(
            rootView: KeyboardView(model: model, showsGlobe: needsInputModeSwitchKey))
        hosting.view.backgroundColor = .clear
        hosting.view.translatesAutoresizingMaskIntoConstraints = false
        addChild(hosting)
        view.addSubview(hosting.view)
        // 键盘高度由我们定，钉在 inputView 上系统才按这个给（否则沿用上一个键盘的高度，内容被居中撑开、整体下移）；
        // 系统旋转或切换时会临时塞一个冲突的高度约束，留一档优先级让它赢
        let height = view.heightAnchor.constraint(equalToConstant: baseHeight)
        height.priority = .defaultHigh
        NSLayoutConstraint.activate([
            hosting.view.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            hosting.view.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            hosting.view.topAnchor.constraint(equalTo: view.topAnchor),
            hosting.view.bottomAnchor.constraint(equalTo: view.bottomAnchor),
            height,
        ])
        heightConstraint = height
        hosting.didMove(toParent: self)
        self.hosting = hosting
    }

    private func mountTouchView() {
        let model = model!
        touchView.onPress = { [touchView] index in
            guard index < touchView.slots.count else { return }
            model.press(slot: index, key: touchView.slots[index].key)
        }
        touchView.onRelease = { [touchView] index, cancelled in
            guard index < touchView.slots.count else { return }
            model.release(slot: index, key: touchView.slots[index].key, cancelled: cancelled)
        }
        touchView.onChevron = { model.toggleCandidatePanel() }
        touchView.onDrag = { [touchView] index, dx in
            guard index < touchView.slots.count else { return }
            model.drag(slot: index, key: touchView.slots[index].key, dx: dx)
        }
        // 面板在触摸层下面：展开时触摸层不认键区的触摸，面板自己收
        panelView.onSelect = { model.selectCandidate($0) }
        view.addSubview(panelView)
        view.addSubview(touchView)
    }

    /// 格子与 SwiftUI 画键用同一个 KeyboardLayout.slots；切层、开关面板、开始 / 结束组字时重算，并盯着下一次变化。
    /// 展开面板时格子清空（面板自己收触摸），⌄ 照常归触摸层，这样才收得起来。
    private func syncTouchView() {
        let size = CGSize(width: view.bounds.width, height: KeyboardView.keyAreaHeight)
        let (layer, panel, composing, hinted) = withObservationTracking {
            (model.layer, model.panel, model.composing, model.hasHintRow)
        } onChange: { [weak self] in
            Task { @MainActor in self?.syncTouchView() }
        }
        let slots = panel == .keys
            ? KeyboardLayout.slots(layer: layer, showsGlobe: needsInputModeSwitchKey, size: size)
            : []
        if slots.map(\.key) != touchView.slots.map(\.key) { touchView.resetTouches() }
        touchView.slots = slots
        let top = hinted ? KeyStyle.hintRowHeight : 0
        touchView.chevron = composing
            ? CGRect(
                x: view.bounds.width - CandidateBar.chevronWidth, y: top,
                width: CandidateBar.chevronWidth, height: KeyStyle.candidateBarHeight)
            : nil
    }

    /// 展开的候选面板是 UIKit 的（SwiftUI 的 ScrollView 在键盘扩展里滑不动）：候选变了就刷新，没展开就藏着。
    private func syncPanel() {
        let (panel, candidates) = withObservationTracking {
            (model.panel, model.candidates)
        } onChange: { [weak self] in
            Task { @MainActor in self?.syncPanel() }
        }
        panelView.isHidden = panel != .candidates
        panelView.candidates = candidates
    }

    /// 进出「恋爱 · 某人」时提示行这一行加上 / 去掉：键盘高度加减一行（0.2 秒），键区与 ⌄ 的触摸范围在
    /// viewDidLayoutSubviews / syncTouchView 里跟着下移。提示本身出现消失不改高度（行一直在，空着而已）。
    private func syncHintRow() {
        let visible = withObservationTracking {
            model.hasHintRow
        } onChange: { [weak self] in
            Task { @MainActor in self?.syncHintRow() }
        }
        let height = baseHeight + (visible ? KeyStyle.hintRowHeight : 0)
        guard let heightConstraint, heightConstraint.constant != height else { return }
        heightConstraint.constant = height
        view.setNeedsLayout()
        UIView.animate(withDuration: 0.2) { [weak self] in
            self?.view.layoutIfNeeded()
        }
    }

    /// 没有提示行时的键盘高度：候选栏加键区。
    private var baseHeight: CGFloat { KeyStyle.candidateBarHeight + KeyboardView.keyAreaHeight }

    /// 提示行占掉的高度：键区与 ⌄ 往下挪这么多。
    private var hintInset: CGFloat { model.hasHintRow ? KeyStyle.hintRowHeight : 0 }

    private var currentSignature: String {
        let modified = SharedStore.cloudFile.flatMap {
            try? FileManager.default.attributesOfItem(atPath: $0.path)[.modificationDate] as? Date
        }
        return "\(hasFullAccess)|\(modified?.timeIntervalSince1970 ?? 0)"
    }

    /// 产品数据随扩展打包在 Data/ 下；设置、连接配置与学习数据的位置见 `UserData`。
    private static func openEngine(fullAccess: Bool) -> Engine? {
        guard let data = Bundle.main.url(forResource: "Data", withExtension: nil) else {
            return nil
        }
        let files = UserData.resolve(fullAccess: fullAccess, bundledData: data)
        return Engine(
            dataDirectory: data, userDirectory: files.userDirectory, configFile: files.configFile,
            cloudConfig: files.cloudFile)
    }
}

