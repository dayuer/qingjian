// 键盘的状态与按键语义，行为对齐 iOS 自带简体拼音：拼音以 marked text 写在宿主光标处，
// 空格上屏首选，换行原样上屏字母（打英文就靠它），组字中敲标点先上屏首选。视图只读状态、转发点击。
// 配了青简 Cloud 时：大模型的候选异步插进候选栏（控制器定时调 poll），没在组字时可以润色光标前的一段话。

import Observation

@MainActor
@Observable
final class KeyboardModel {
    private(set) var preedit = ""

    private(set) var candidates: [CandidateItem] = []

    private(set) var rewrite = RewriteState.idle

    private(set) var layer = KeyLayer.letters

    private(set) var panel = KeyboardPanel.keys

    private(set) var shifted = false

    /// 引擎打不开（数据缺失）时为 nil，字母直接输出。
    @ObservationIgnored private let engine: Engine?

    @ObservationIgnored weak var output: TextOutput?

    /// 每次按键按下时调（键盘音与震动），由控制器接上。
    @ObservationIgnored var onKeyDown: (() -> Void)?

    init(engine: Engine?) {
        self.engine = engine
    }

    var composing: Bool { !preedit.isEmpty }

    var rewriteAvailable: Bool { engine?.rewriteAvailable ?? false }

    func tap(_ key: Key) {
        onKeyDown?()
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
        engine?.clear()
        engine?.flush()
        panel = .keys
        refresh()
    }

    /// 键盘出现：先同步一轮，别的设备学到的词尽快过来。
    func appear() {
        engine?.syncNow()
    }

    /// 控制器定时调：取大模型候选、合并别的设备的学习数据、看润色有没有回来。
    func poll() {
        guard let engine else { return }
        if engine.poll() { candidates = engine.candidates }
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
        if let output { engine.setContext(before: output.contextBefore, after: output.contextAfter) }
        engine.push(letter)
        refresh()
    }

    private func typeSymbol(_ text: String) {
        commitFirst()
        output?.commit(text)
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
    }
}
