// 键盘扩展入口：打开引擎、挂上 SwiftUI 键盘，把输出接到 textDocumentProxy（拼音走 marked text）。

import SwiftUI
import UIKit
import os

final class KeyboardViewController: UIInputViewController, TextOutput {
    private var model: KeyboardModel!

    private var hosting: UIHostingController<KeyboardView>?

    /// 宿主里现在有没有我们写的 marked text；proxy 没有接口能问，自己记。
    private var hasMarkedText = false

    private let feedback = KeyFeedback()

    /// 键区的触摸层（SwiftUI 只画键）；展开候选或表情面板时藏起来。
    private let touchView = KeyTouchView()

    /// 展开的候选面板（UIKit，见 `applyPanel`）。
    private let panelView = CandidatePanelView()

    /// 候选栏里横向滚动的那一行（UIKit，见 `applyBar`）：SwiftUI 的 ScrollView 在键盘扩展里收不到滑动。
    private let barView = CandidateBarView()

    /// 上一次的拼音：变了就把候选栏滚回开头（云端词插进来时不算变，别跳）。
    private var lastPreedit = ""

    /// 键盘可见期间每 0.25 秒一次：大模型候选、润色结果、别的设备的学习数据都靠它取回。
    private var pollTimer: Timer?

    /// 打开引擎时的完全访问与 cloud.toml 修改时间；出现时对不上就重开引擎。
    private var engineSignature = ""

    /// 钉在 inputView 上的键盘高度；进出「恋爱 · 某人」时加减一行提示行（见 `syncHintRow`）。
    private var heightConstraint: NSLayoutConstraint?

    /// 上一次看到的宿主输入框（`textDocumentProxy.documentIdentifier`）。
    private var lastDocument: UUID?

    /// 系统在控制器走了以后还留着它的根视图（inputView），挂在上面的 SwiftUI 宿主视图、触摸层、候选栏也就一直活着，
    /// 宿主视图里的 KeyboardView 又强引用模型 → 引擎（Rust 会话、模型映射、打分线程）：键盘每出现一次就漏一整份。
    /// 所以控制器析构时把根视图上的子视图全摘掉、回调清空。deinit 不在主线程隔离里，摘视图排回主线程做。
    deinit {
        LiveCount.released("controller")
        let (root, touch, panel, bar) = (view, touchView, panelView, barView)
        DispatchQueue.main.async {
            MainActor.assumeIsolated {
                touch.onPress = nil
                touch.onRelease = nil
                touch.onDrag = nil
                touch.onChevron = nil
                panel.onSelect = nil
                bar.onSelect = nil
                root?.subviews.forEach { $0.removeFromSuperview() }
            }
        }
    }

    override func loadView() {
        super.loadView()
        inputView = KeyboardInputView(frame: .zero, inputViewStyle: .keyboard)
    }

    override func viewDidLoad() {
        super.viewDidLoad()
        engineSignature = currentSignature
        LiveCount.created("controller")
        PerfRecorder.shared.mark("before_engine")
        model = KeyboardModel(engine: Self.openEngine(fullAccess: hasFullAccess))
        PerfRecorder.shared.mark("after_engine")
        model.output = self
        model.onKeyDown = { [feedback] in feedback.keyDown() }
        mountKeyboard()
        mountTouchView()
        syncHintRow()
        Watched.allCases.forEach(observe)
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
        barView.frame = CGRect(
            x: 0, y: hintInset, width: view.bounds.width, height: KeyStyle.candidateBarHeight)
        // 只套用，不注册观察：布局每键都走，在这里注册会越积越多（见 observe）
        applyTouchView()
        applyPanel()
        applyBar()
    }

    /// 屏幕左右边缘的触摸会被系统边缘手势压住（a、l 慢半拍或丢）：要我们先处理。iOS 在视图切换时会重置，所以 viewWillAppear 再要一次。
    override var preferredScreenEdgesDeferringSystemGestures: UIRectEdge { [.left, .right] }

    override func viewWillAppear(_ animated: Bool) {
        super.viewWillAppear(animated)
        // 完全访问用来震动与连素笺云；用户随时可能去设置里开关，每次出现时重读
        feedback.hapticsEnabled = hasFullAccess
        setNeedsUpdateOfScreenEdgesDeferringSystemGestures()
        if currentSignature != engineSignature {
            engineSignature = currentSignature
            // 先释放旧引擎再建新的，只是缩小新旧 DataSync 线程重叠的窗口（旧线程空闲或在等待时，会在新线程起来前收到停止信号），
            // 并没有消除：DataSync 的 Drop 只置 stop 不 join，旧线程若正好在一轮 HTTP 里，结束时仍可能把进度文件写回。
            // 学习数据那一项每轮拿文件锁、重读基线，重叠也不会重复计数；配置与输入日志的进度仍是已知限制
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
        model.uploadKick()
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
            // 真机核对换输入框时标识是否变化、上下文是否清掉；标识是系统给的随机 UUID，不含输入内容
            let from = Self.short(lastDocument), to = Self.short(document)
            Self.log.info("宿主输入框切换 \(from, privacy: .public) → \(to, privacy: .public)，清上下文")
            lastDocument = document
            // 旧输入框里的 marked text 已由系统收尾，这里只忘掉它；否则清组字时的 setMarked("") 会写进新输入框
            hasMarkedText = false
            model.hostChanged()
        }
    }

    /// 宿主输入框的标识。`textDocumentProxy.documentIdentifier` 声明为非可选 UUID，但键盘刚弹出、连上宿主之前系统返回 nil，
    /// 直接读会在 UUID 桥接处 EXC_BREAKPOINT 崩溃，所以走 KVC 取成可选值，别「简化」回去。
    private static let log = Logger(subsystem: "sujian.synon.ai.keyboard", category: "host")

    private static func short(_ id: UUID?) -> String { id.map { String($0.uuidString.prefix(8)) } ?? "nil" }

    private var hostDocumentIdentifier: UUID? {
        (textDocumentProxy as? NSObject)?.value(forKey: "documentIdentifier") as? UUID
    }

    var contextBefore: String { textDocumentProxy.documentContextBeforeInput ?? "" }

    var selectedText: String? { textDocumentProxy.selectedText }

    var contextAfter: String { textDocumentProxy.documentContextAfterInput ?? "" }

    func switchToNextKeyboard() {
        advanceToNextInputMode()
    }

    var pasteboardChangeCount: Int { UIPasteboard.general.changeCount }

    var pasteboardHasText: Bool { UIPasteboard.general.hasStrings }

    func readPasteboard() -> String? { UIPasteboard.general.string }

    /// 密码框（secureTextEntry）系统根本不给第三方键盘；这里挡的是验证码、新密码、信用卡号这类照样用我们键盘的字段。
    /// 私密输入 = 不记输入日志、不看宿主前文（`MutedLogger` 在桥里把日志整条静音）。
    /// 电话号码与纯数字键盘（验证码、金额）也算：这类框里敲的字不该进任何记录。
    private func updatePrivacy() {
        let proxy = textDocumentProxy
        let secure = proxy.isSecureTextEntry ?? false
        let sensitive = proxy.textContentType.map(Self.sensitiveContentTypes.contains) ?? false
        let numericPad = [
            UIKeyboardType.numberPad, .phonePad, .decimalPad,
        ].contains(proxy.keyboardType ?? .default)
        model.setPrivateField(secure || sensitive || numericPad)
    }

    private static let sensitiveContentTypes: Set<UITextContentType> = [
        .password, .newPassword, .oneTimeCode, .creditCardNumber,
        .creditCardSecurityCode, .creditCardExpiration, .creditCardExpirationMonth,
        .creditCardExpirationYear, .telephoneNumber,
    ]

    private func mountKeyboard() {
        let hosting = UIHostingController(
            rootView: KeyboardView(model: model, showsGlobe: needsInputModeSwitchKey))
        hosting.view.backgroundColor = .clear
        hosting.view.translatesAutoresizingMaskIntoConstraints = false
        addChild(hosting)
        view.addSubview(hosting.view)
        // 键盘高度由我们定，viewDidLoad 里就按最终高度钉在 inputView 上（配合 KeyboardInputView.allowsSelfSizing），
        // 系统第一次就按这个给；优先级 999：750 时会先沿用上一个键盘（如系统英文键盘）的高度、出现后才改，
        // 微信这类宿主按旧高度布局，输入栏下半截被盖住。留一档给系统旋转时临时塞的冲突约束
        let height = view.heightAnchor.constraint(equalToConstant: baseHeight)
        height.priority = UILayoutPriority(999)
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

    /// 回调都弱引用触摸层与模型：闭包存在触摸层自己身上，强引用触摸层就是自己留住自己，
    /// 连带模型与引擎（Rust 会话、模型映射、打分线程）在键盘每次收起后都漏一份。
    private func mountTouchView() {
        let model = model!
        touchView.onPress = { [weak touchView, weak model] index in
            guard let touchView, let model, index < touchView.slots.count else { return }
            model.press(slot: index, key: touchView.slots[index].key)
        }
        touchView.onRelease = { [weak touchView, weak model] index, cancelled in
            guard let touchView, let model, index < touchView.slots.count else { return }
            model.release(slot: index, key: touchView.slots[index].key, cancelled: cancelled)
        }
        touchView.onChevron = { [weak model] in model?.toggleCandidatePanel() }
        touchView.onDrag = { [weak touchView, weak model] index, dx in
            guard let touchView, let model, index < touchView.slots.count else { return }
            model.drag(slot: index, key: touchView.slots[index].key, dx: dx)
        }
        // 面板在触摸层下面：展开时触摸层不认键区的触摸，面板自己收
        panelView.onSelect = { [weak model] in model?.selectCandidate($0) }
        barView.onSelect = { [weak model] in model?.selectCandidate($0) }
        view.addSubview(panelView)
        // 候选栏也在触摸层下面：触摸层的 hitTest 只认键区与 ⌄，候选栏那一条会穿透下来，滑动与点击都归它
        view.addSubview(barView)
        view.addSubview(touchView)
    }

    /// 跟着 model 变的三处 UIKit 视图（触摸层、展开面板、候选栏）：`observe` 只在 viewDidLoad 起一次，之后由自己的 onChange 续上；
    /// 布局时只调 `applyX`。`withObservationTracking` 的注册不会撤销，在每键都走的 viewDidLayoutSubviews 里注册会越积越多，
    /// 一次候选变化同时触发几十上百个主线程任务（2026-10-06 真机打点：每键 37–145 次，主线程每键多忙约 20ms）。
    /// onChange 在值改之前触发、只触发一次、不保证在主线程：先回主线程，到那时值已改好，再在注册闭包里套用（读到新值）并重新注册。
    private enum Watched: CaseIterable { case touchView, panel, bar }

    /// 每条链此刻有没有挂着注册：同一条链只能挂一个，挂第二个就是又漏回去了。
    private var armed: Set<Watched> = []

    private func observe(_ watched: Watched) {
        assert(!armed.contains(watched), "\(watched) 的观察重复注册")
        armed.insert(watched)
        withObservationTracking {
            switch watched {
            case .touchView: applyTouchView()
            case .panel: applyPanel()
            case .bar: applyBar()
            }
        } onChange: { [weak self] in
            DispatchQueue.main.async {
                MainActor.assumeIsolated {
                    guard let self else { return }
                    self.armed.remove(watched)
                    self.observe(watched)
                }
            }
        }
    }

    /// 格子与 SwiftUI 画键用同一个 KeyboardLayout.slots；切层、开关面板、开始 / 结束组字时重算。
    /// 展开面板时格子清空（面板自己收触摸），⌄ 照常归触摸层，这样才收得起来。
    private func applyTouchView() {
        let size = CGSize(width: view.bounds.width, height: KeyboardView.keyAreaHeight)
        let (layer, panel, composing, top) = (model.layer, model.panel, model.composing, model.hintRowHeight)
        let slots = panel == .keys
            ? KeyboardLayout.slots(layer: layer, showsGlobe: needsInputModeSwitchKey, size: size)
            : []
        if slots.map(\.key) != touchView.slots.map(\.key) { touchView.resetTouches() }
        touchView.slots = slots
        // 对象卡与选择面板打开时候选栏那一行换成它们的工具栏，没有 ⌄（CandidateBar）
        let showsChevron = composing && !CandidateBar.panelTakesTheBar(panel)
        touchView.chevron = showsChevron
            ? CGRect(
                x: view.bounds.width - CandidateBar.chevronWidth, y: top,
                width: CandidateBar.chevronWidth, height: KeyStyle.candidateBarHeight)
            : nil
    }

    /// 展开的候选面板是 UIKit 的（SwiftUI 的 ScrollView 在键盘扩展里滑不动）：展开时给它当前候选，没展开就藏着、清空。
    /// 藏着的面板照样占整块键区参与布局，若每键都给它候选，每键都要重排一屏格子；展开那一刻 panel 变了会再走这里，拿到的是最新候选。
    private func applyPanel() {
        let (panel, candidates) = (model.panel, model.candidates)
        let shown = panel == .candidates
        panelView.isHidden = !shown
        panelView.candidates = shown ? candidates : []
    }

    /// 候选栏里横向滚动的那一行。组字时显示；对象卡与选择面板打开时藏起来（那一行换成它们的工具栏）。
    /// 位置跟着提示行走（提示行出现时候选栏往下挪一行），所以 viewDidLayoutSubviews 里也重设一次 frame。
    private func applyBar() {
        let (composing, panel, candidates, accentFirst, preedit) =
            (model.composing, model.panel, model.candidates, model.accentFirstCandidate, model.preedit)
        // 展开面板与候选栏是同一套样式，首选那个的强调色一起同步
        panelView.accentFirst = accentFirst
        barView.isHidden = !(composing && !CandidateBar.panelTakesTheBar(panel))
        // 拼音变了从头显示；只是云端词插进来时别跳
        let fromStart = preedit != lastPreedit
        lastPreedit = preedit
        barView.show(candidates, accentFirst: accentFirst, fromStart: fromStart)
    }

    /// 提示行出现与收起时（hintRowHeight：提示行 34、记一笔确认条更高）键盘高度跟着加减（0.2 秒），键区与 ⌄ 的触摸范围在
    /// viewDidLayoutSubviews / applyTouchView 里跟着下移。
    private func syncHintRow() {
        let hintHeight = withObservationTracking {
            model.hintRowHeight
        } onChange: { [weak self] in
            Task { @MainActor in self?.syncHintRow() }
        }
        let height = baseHeight + hintHeight
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
    private var hintInset: CGFloat { model.hintRowInset }

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
        let engine = Engine(
            dataDirectory: data, userDirectory: files.userDirectory, configFile: files.configFile,
            cloudConfig: files.cloudFile)
        engine?.loadModel()
        return engine
    }
}

