// 键盘的状态与按键语义，行为对齐 iOS 自带简体拼音：拼音以 marked text 写在宿主光标处，
// 空格上屏首选，换行原样上屏字母（打英文就靠它），组字中敲标点先上屏首选。视图只读状态、转发点击。
// 配了青简 Cloud 时：大模型的候选异步插进候选栏（控制器定时调 poll），没在组字时可以润色光标前的一段话、
// 插入别的设备刚复制的文字、把本机剪贴板发出去。验证码 / 密码这类输入框里这些都停（privateField）。
// 本地记忆：场景牌子与选择面板、候选栏上方的提示行、对象卡、「记一笔」都经 MemoryBridge 调桥；名单读 App Group 里的 memory/。

import Foundation
import Observation

@MainActor
@Observable
final class KeyboardModel {
    private(set) var preedit = ""

    private(set) var candidates: [CandidateItem] = []

    private(set) var rewrite = RewriteState.idle

    /// 别的设备刚复制、还没处理的文字。
    private(set) var clipOffer: ClipOffer?

    /// 本机剪贴板自上次处理后变过（有文字）：候选栏给「发到其他设备」。
    private(set) var pasteboardChanged = false

    /// 焦点在验证码、密码、信用卡号这类输入框。
    private(set) var privateField = false

    /// 记忆的提示行：恋爱场景、选了对象、碰上卡片里的词或日子快到时有。
    private(set) var hint: MemoryHint?

    /// 当前场景与对象（只能用户自己切）。
    private(set) var scope = MemoryScope()

    /// App 里建的恋爱场景的人；开了完全访问才读得到 App Group。
    private(set) var contacts: [MemoryContact] = []

    /// 「记一笔」确认条里的剪贴板文字；nil 时不显示。
    private(set) var noteDraft: String?

    /// 「记一笔」刚记下：确认条换成一行「记下了」，2 秒后消失。
    private(set) var noteDone = false

    /// 面板里的一行短提示（键盘扩展打不开 App，「全部记忆」「去开启」只能这样告诉用户），2 秒后消失。
    private(set) var notice: String?

    /// 当前对象之外的所有恋爱对象的卡片，按卡片 id 查（提示行加粗关键词、来源标签用）。
    @ObservationIgnored private var cardIndex: [String: MemoryCard] = [:]

    /// 对象卡面板里的卡片。
    private(set) var panelCards: [MemoryCard] = []

    /// 本机剪贴板里有没有文字（键盘出现时看一次；只看不读，不弹粘贴授权）。
    private(set) var clipboardHasText = false

    /// 开了完全访问：读 App Group 里的名单、「记一笔」读剪贴板都要它。控制器每次出现时设。
    var fullAccess = false

    private(set) var layer = KeyLayer.letters

    private(set) var panel = KeyboardPanel.keys

    private(set) var shifted = false

    /// 正按着的格子（KeyboardLayout.slots 的下标），键帽据此变色、弹放大字样。
    private(set) var pressedSlots: Set<Int> = []

    /// 按住 ⌫ 连删的任务，按格子记。
    @ObservationIgnored private var repeats: [Int: Task<Void, Never>] = [:]

    /// 按住空格拖动挪光标：哪一格进了这个模式、已经挪了几步。抬起时不出空格。
    @ObservationIgnored private var cursorDrag: (slot: Int, steps: Int)?


    /// 引擎打不开（数据缺失）时为 nil，字母直接输出。
    @ObservationIgnored private var engine: Engine?

    @ObservationIgnored weak var output: TextOutput?

    /// 每次按键按下时调（键盘音与震动），由控制器接上。
    @ObservationIgnored var onKeyDown: (() -> Void)?

    init(engine: Engine?) {
        self.engine = engine
    }

    /// 换一个引擎（完全访问开关或连接配置变了，学习数据目录与云端都要重开）；旧的先落盘。
    func replaceEngine(_ engine: Engine?) {
        self.engine?.flush()
        self.engine = engine
        if privateField { engine?.setPrivate(true) }
        let shown = EngineDisplay.afterReplace(hasEngine: engine != nil, preedit: preedit, candidates: candidates)
        if shown.preedit != preedit { output?.setMarked(shown.preedit) }
        preedit = shown.preedit
        candidates = shown.candidates
        refresh()
        syncScope()
    }

    var composing: Bool { !preedit.isEmpty }

    var rewriteAvailable: Bool { engine?.rewriteAvailable ?? false }

    func tap(_ key: Key) {
        // 又开始打字了：没用上的润色作废
        if rewrite != .idle, key != .shift, key != .globe { dismissRewrite() }
        switch key {
        case .letter(let letter): typeLetter(letter)
        case .symbol(let text): typeSymbol(text)
        case .shift: shifted.toggle()
        case .backspace: backspace()
        case .layer(let target): layer = target
        case .globe: output?.switchToNextKeyboard()
        case .emoji: panel = .emoji
        case .space: space()
        case .returnKey: returnKey()
        }
    }

    /// 按下：高亮、反馈；⌫ 按下即删，按住 0.4 秒后每 0.08 秒连删一次。别的键抬起才出字（与系统键盘一致）。
    func press(slot: Int, key: Key) {
        guard !pressedSlots.contains(slot) else { return }
        pressedSlots.insert(slot)
        onKeyDown?()
        guard key == .backspace else { return }
        tap(key)
        repeats[slot] = Task { @MainActor [weak self] in
            try? await Task.sleep(for: .milliseconds(400))
            while !Task.isCancelled {
                self?.onKeyDown?()
                self?.tap(.backspace)
                try? await Task.sleep(for: .milliseconds(80))
            }
        }
    }

    /// 抬起出字。被系统取消的触摸（屏幕边缘手势抢的）也按抬起算：用户确实按了这个键。
    func release(slot: Int, key: Key, cancelled: Bool) {
        guard pressedSlots.remove(slot) != nil else { return }
        repeats.removeValue(forKey: slot)?.cancel()
        if cursorDrag?.slot == slot {
            cursorDrag = nil
            return
        }
        if key != .backspace { tap(key) }
    }

    /// 按住空格横向拖：挪过 [`Self.cursorDragStart`] 进挪光标模式，之后每 [`Self.cursorStep`] 挪一个字，每步轻震一下。
    /// 组字中不挪（光标在拼音里没意义），照常当空格。
    func drag(slot: Int, key: Key, dx: CGFloat) {
        guard key == .space, pressedSlots.contains(slot), !composing else { return }
        if cursorDrag == nil {
            guard abs(dx) >= Self.cursorDragStart else { return }
            cursorDrag = (slot, 0)
        }
        guard var drag = cursorDrag, drag.slot == slot else { return }
        let travelled = dx - (dx > 0 ? Self.cursorDragStart : -Self.cursorDragStart)
        let steps = Int(travelled / Self.cursorStep)
        guard steps != drag.steps else { return }
        output?.moveCursor(by: steps - drag.steps)
        onKeyDown?()
        drag.steps = steps
        cursorDrag = drag
    }

    /// 横向挪多远才算拖（pt），免得点空格时手指一抖就挪了光标。
    private static let cursorDragStart: CGFloat = 12

    /// 拖多远挪一个字（pt）。
    private static let cursorStep: CGFloat = 9

    func selectCandidate(_ index: Int) {
        guard let engine, let text = engine.commit(index) else { return }
        output?.commit(text)
        refresh()
        // 候选吃完了拼音就收起展开的候选面板；还剩拼音时留着接着选
        if !composing, panel == .candidates { panel = .keys }
    }

    func toggleCandidatePanel() {
        panel = panel == .candidates ? .keys : .candidates
    }

    func closePanel() {
        panel = .keys
    }

    func typeEmoji(_ emoji: String) {
        commitFirst()
        output?.commit(emoji)
    }

    /// 键盘收起：没上屏的拼音丢掉，学习数据落盘并催一轮同步。
    func dismiss() {
        dismissRewrite()
        noteDraft = nil
        noteDone = false
        notice = nil
        engine?.clear()
        engine?.flush()
        panel = .keys
        refresh()
    }

    /// 键盘出现：先同步一轮，别的设备学到的词尽快过来；拉一次别的设备的剪贴板，看本机剪贴板变没变。
    func appear() {
        engine?.syncNow()
        engine?.refreshClipboard()
        checkPasteboard()
        clipboardHasText = output?.pasteboardHasText ?? false
        syncScope()
    }

    /// 焦点换到了（不）是验证码 / 密码这类输入框。
    func setPrivateField(_ value: Bool) {
        guard value != privateField else { return }
        privateField = value
        engine?.setPrivate(value)
        if value {
            dismissRewrite()
            clipOffer = nil
            pasteboardChanged = false
            noteDraft = nil
        } else {
            checkPasteboard()
        }
        refreshHint()
    }

    func insertClip() {
        guard let offer = clipOffer else { return }
        commitFirst()
        output?.commit(offer.text)
        engine?.clipHandled()
        clipOffer = nil
    }

    func dismissClip() {
        engine?.clipHandled()
        clipOffer = nil
    }

    /// 读本机剪贴板（可能弹系统的粘贴授权提示）发给别的设备。
    func pushPasteboard() {
        guard let engine, let output, !privateField else { return }
        if let text = output.readPasteboard(), !text.isEmpty {
            engine.pushClip(text)
        }
        markPasteboardSeen()
    }

    func dismissPasteboard() {
        markPasteboardSeen()
    }

    private func checkPasteboard() {
        guard let engine, engine.clipboardEnabled, let output, !privateField else { return }
        let count = output.pasteboardChangeCount
        // 第一次用：装键盘之前就在剪贴板里的不算新复制的
        guard let seen = Self.seenPasteboardCount else {
            Self.seenPasteboardCount = count
            return
        }
        pasteboardChanged = count != seen && output.pasteboardHasText
    }

    private func markPasteboardSeen() {
        if let output { Self.seenPasteboardCount = output.pasteboardChangeCount }
        pasteboardChanged = false
    }

    /// 上次处理过的剪贴板变化计数，存在扩展自己的 UserDefaults 里（键盘进程随时被杀）。
    private static var seenPasteboardCount: Int? {
        get { UserDefaults.standard.object(forKey: "seenPasteboardCount") as? Int }
        set { UserDefaults.standard.set(newValue, forKey: "seenPasteboardCount") }
    }

    /// 控制器定时调：取大模型候选、合并别的设备的学习数据、看润色有没有回来。
    func poll() {
        guard let engine else { return }
        refreshHint()
        // App 删了当前对象时桥会退回「恋爱 · 不指定」
        if let next = engine.scope, next != scope {
            scope = next
            reloadContacts()
        }
        if engine.poll() { candidates = engine.candidates }
        if !privateField {
            let offer = engine.clipOffer
            if offer != clipOffer { clipOffer = offer }
        }
        guard case .pending(let original) = rewrite else { return }
        switch engine.rewriteStatus {
        case 2:
            if let result = engine.takeRewrite() {
                rewrite = .ready(original: original, result: result)
            }
        case 3:
            rewrite = .failed
        default:
            break
        }
    }

    /// 润色光标前的一段话（从上一个换行起，最多 300 字）。
    func startRewrite() {
        guard let engine, !composing, let output else { return }
        let paragraph = output.contextBefore.split(separator: "\n", omittingEmptySubsequences: false)
            .last.map(String.init) ?? ""
        let original = String(paragraph.suffix(300))
        guard !original.trimmingCharacters(in: .whitespaces).isEmpty else { return }
        engine.startRewrite(original)
        rewrite = .pending(original: original)
    }

    /// 用润色结果替换原文。光标前已经不是原文了（用户挪了光标或改了字）就放弃，不乱删。
    func applyRewrite() {
        guard case .ready(let original, let result) = rewrite, let output else { return }
        rewrite = .idle
        guard output.contextBefore.hasSuffix(original) else { return }
        for _ in original { output.deleteBackward() }
        output.commit(result)
    }

    func dismissRewrite() {
        engine?.cancelRewrite()
        rewrite = .idle
    }

    /// 提示行这一行在不在：恋爱场景且选了对象时一直在（没有提示时是空行），日常、工作与「恋爱 · 不指定」没有这一行。
    /// 键盘高度只在进出这个状态时变，提示出现与消失不再让宿主界面跳。
    var hasHintRow: Bool { ScopeDisplay.hasHintRow(scene: scope.scene, hasContact: currentContact != nil) }

    /// 宿主换了输入框（控制器按 documentIdentifier 判断）：清掉最近上屏的字与提示。
    func hostChanged() {
        engine?.resetContext()
        refreshHint()
    }

    /// 当前对象（名单里找得到的）。
    var currentContact: MemoryContact? {
        guard let id = scope.contactId else { return nil }
        return contacts.first { $0.id == id }
    }

    /// 「记一笔」：开了完全访问、剪贴板有字、选了对象、不在私密输入框。
    var canNote: Bool {
        ScopeDisplay.canNote(
            fullAccess: fullAccess, clipboardHasText: clipboardHasText, privateField: privateField,
            hasContact: currentContact != nil)
    }

    func openScopePicker() {
        reloadContacts()
        panel = .scope
    }

    /// 选场景与对象。选了对象或换到日常 / 工作就收起面板；换到恋爱还没选对象时留着接着选。
    func chooseScope(scene: String, contactId: String?) {
        guard let engine else { return }
        engine.setScope(scene: scene, contactId: contactId)
        scope = engine.scope ?? scope
        refresh()
        if scene != MemoryScope.dating || contactId != nil { panel = .keys }
    }

    func openContactCard() {
        guard let id = scope.contactId else { return }
        panelCards = engine?.memoryCards(id) ?? []
        panel = .contactCard
    }

    /// 日子提醒的「知道了」：当天不再出。
    func acknowledgeHint() {
        guard let hint else { return }
        engine?.dismissHint(hint.cardId, today: hint.reason == .today)
        refreshHint()
    }

    /// 点「记一笔」：读剪贴板（可能弹系统的粘贴授权），显示确认条。
    func startNote() {
        guard canNote,
              let text = output?.readPasteboard()?.trimmingCharacters(in: .whitespacesAndNewlines),
              !text.isEmpty
        else { return }
        // 一张卡最多 200 字（桥也会校验），长的剪贴板截掉后面的，确认条里看得到截后的样子
        noteDraft = MemoryLimits.clampText(text)
    }

    func confirmNote() {
        guard let text = noteDraft, let id = scope.contactId else { return }
        noteDraft = nil
        // nil 即成功（含桥「已接受、稍后写入」）；写不进（App Group 不可写、对象刚被删）时不弹错，不打断打字
        if engine?.memoryNote(id, text: text) == nil {
            noteDone = true
            noteDoneTask?.cancel()
            noteDoneTask = Task { @MainActor [weak self] in
                try? await Task.sleep(for: .seconds(2))
                guard !Task.isCancelled else { return }
                self?.noteDone = false
            }
        }
        reloadContacts()
        refreshHint()
    }

    func cancelNote() {
        noteDraft = nil
    }

    /// 提示行里要加粗的词。
    var hintEmphasis: [String] {
        guard let hint else { return [] }
        return HintText.emphasis(for: hint, card: cardIndex[hint.cardId])
    }

    /// 提示对应卡片的来源（手动卡显示「你写的」）。
    var hintSource: String? { hint.flatMap { cardIndex[$0.cardId]?.source } }

    /// 面板里显示一行 2 秒的短提示。
    func showNotice(_ text: String) {
        notice = text
        noticeTask?.cancel()
        noticeTask = Task { @MainActor [weak self] in
            try? await Task.sleep(for: .seconds(2))
            guard !Task.isCancelled else { return }
            self?.notice = nil
        }
    }

    @ObservationIgnored private var noticeTask: Task<Void, Never>?
    @ObservationIgnored private var noteDoneTask: Task<Void, Never>?

    /// 换了引擎、键盘出现时：从桥取当前场景，重读名单与提示。
    private func syncScope() {
        scope = engine?.scope ?? MemoryScope()
        reloadContacts()
        refreshHint()
    }

    private func reloadContacts() {
        guard fullAccess, let directory = SharedStore.directory,
              let snapshot = MemoryFiles.read(userDirectory: directory)
        else {
            contacts = []
            cardIndex = [:]
            return
        }
        contacts = snapshot.contacts.filter { $0.scene == MemoryScope.dating }
        cardIndex = Dictionary(
            snapshot.cards.values.joined().map { ($0.id, $0) }, uniquingKeysWith: { first, _ in first })
    }

    private func refreshHint() {
        let next = privateField ? nil : engine?.memoryHint
        // 提示的卡是刚记下的、名单缓存里还没有时，重读一次再显示
        if let next, cardIndex[next.cardId] == nil { reloadContacts() }
        if next != hint { hint = next }
    }

    private func typeLetter(_ letter: Character) {
        let upper = shifted
        shifted = false
        guard let engine else {
            output?.commit(upper ? letter.uppercased() : String(letter))
            return
        }
        // Shift 字母是临时打英文：组到一半的拼音先原样上屏
        if upper {
            if composing { output?.commit(engine.takeRaw()) }
            output?.commit(letter.uppercased())
            refresh()
            return
        }
        // 光标前后文只在开始组字时取一次：每次都问宿主是一次跨进程往返，每个键都要等
        if !composing, let output {
            engine.setContext(before: output.contextBefore, after: output.contextAfter)
        }
        engine.push(letter)
        refresh()
    }

    /// 敲了断句的标点就回字母层，接着打拼音（与系统键盘一致）；数字与 `- / : . @` 这类常夹在数字里的符号留在原层，连着敲。
    private static let returnsToLetters: Set<String> = ["。", "，", "、", "？", "！", "；", "…", "”", "’", "^_^"]

    private func typeSymbol(_ text: String) {
        commitFirst()
        output?.commit(text)
        if Self.returnsToLetters.contains(text) { layer = .letters }
    }

    private func backspace() {
        if composing, let engine {
            engine.backspace()
            refresh()
        } else {
            output?.deleteBackward()
        }
    }

    private func space() {
        if composing {
            commitFirst()
        } else {
            engine?.notePassthrough(" ")
            output?.commit(" ")
        }
        layer = .letters
    }

    private func returnKey() {
        if composing, let engine {
            output?.commit(engine.takeRaw())
            refresh()
        } else {
            engine?.notePassthrough("\n")
            output?.commit("\n")
        }
    }

    /// 上屏首选；拼不成音节（没有候选）时原样上屏。候选只吃掉一部分拼音时，剩下的接着组。
    private func commitFirst() {
        guard composing, let engine else { return }
        let text = candidates.isEmpty ? engine.takeRaw() : (engine.commit(0) ?? engine.takeRaw())
        output?.commit(text)
        refresh()
    }

    private func refresh() {
        guard let engine else { return }
        let next = engine.preedit
        if next != preedit { output?.setMarked(next) }
        preedit = next
        candidates = engine.candidates
        if !composing, panel == .candidates { panel = .keys }
        refreshHint()
    }
}
