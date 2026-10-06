// KeyboardModel 唯一的输出口：平时原样转给宿主（控制器），手写记一笔时把上屏、退格改道到草稿（NoteComposer），
// 写宿主的接口一个都不碰。KeyboardModel 不直接持有宿主，所以「手写时宿主里不多不少」只靠这里守，单测也测这里。

import Observation

@MainActor
@Observable
final class OutputRouter: TextOutput {
    @ObservationIgnored weak var host: TextOutput?

    /// 手写记一笔的草稿；nil 时不在手写状态。
    private(set) var composer: NoteComposer?

    var isComposingNote: Bool { composer != nil }

    func beginNote() {
        composer = NoteComposer()
    }

    /// 退出手写，草稿丢掉；要存的话先取 `composer`。
    func endNote() {
        composer = nil
    }

    /// 手写时拼音不进宿主的 marked text（草稿条自己画），这里什么都不做。
    func setMarked(_ text: String) {
        guard composer == nil else { return }
        host?.setMarked(text)
    }

    func commit(_ text: String) {
        if composer != nil {
            composer?.append(text)
        } else {
            host?.commit(text)
        }
    }

    func deleteBackward() {
        if composer != nil {
            composer?.deleteBackward()
        } else {
            host?.deleteBackward()
        }
    }

    /// 草稿的光标只在末尾。
    func moveCursor(by offset: Int) {
        guard composer == nil else { return }
        host?.moveCursor(by: offset)
    }

    /// 手写时给引擎的上下文是草稿本身，不去问宿主。
    var contextBefore: String { composer?.text ?? host?.contextBefore ?? "" }

    var selectedText: String? { composer == nil ? host?.selectedText : nil }

    var contextAfter: String { composer == nil ? host?.contextAfter ?? "" : "" }

    func switchToNextKeyboard() { host?.switchToNextKeyboard() }

    func dismissKeyboard() { host?.dismissKeyboard() }

    var pasteboardChangeCount: Int { host?.pasteboardChangeCount ?? 0 }

    var pasteboardHasText: Bool { host?.pasteboardHasText ?? false }

    func readPasteboard() -> String? { host?.readPasteboard() }
}
