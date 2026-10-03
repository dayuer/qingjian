// 键盘的状态与按键语义，行为对齐 iOS 自带简体拼音：拼音以 marked text 写在宿主光标处，
// 空格上屏首选，换行原样上屏字母（打英文就靠它），组字中敲标点先上屏首选。视图只读状态、转发点击。

import Observation

@MainActor
@Observable
final class KeyboardModel {
    private(set) var preedit = ""

    private(set) var candidates: [String] = []

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

    func tap(_ key: Key) {
        onKeyDown?()
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

    /// 键盘收起：没上屏的拼音丢掉，学习数据落盘。
    func dismiss() {
        engine?.clear()
        engine?.flush()
        panel = .keys
        refresh()
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
            output?.commit(" ")
        }
    }

    private func returnKey() {
        if composing, let engine {
            output?.commit(engine.takeRaw())
            refresh()
        } else {
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
