// 手写记一笔：草稿的追加、退格、200 字截断、能不能记、占位文字；入口怎么选路；
// 以及硬性要求——手写时 OutputRouter 一次都不调宿主的 setMarked / commit / deleteBackward / moveCursor。
// KeyboardModel 依赖桥与 App Group 进不了测试 target，它所有输出都经 OutputRouter、不直接持有宿主，所以守住路由器就守住了宿主。

import XCTest
@testable import QingjianCloud

@MainActor
private final class FakeOutput: TextOutput {
    var setMarkedCalls: [String] = []

    var commitCalls: [String] = []

    var deleteCalls = 0

    var moveCalls: [Int] = []

    var writes: Int { setMarkedCalls.count + commitCalls.count + deleteCalls + moveCalls.count }

    func setMarked(_ text: String) { setMarkedCalls.append(text) }

    func commit(_ text: String) { commitCalls.append(text) }

    func deleteBackward() { deleteCalls += 1 }

    func moveCursor(by offset: Int) { moveCalls.append(offset) }

    var contextBefore = "宿主里光标前的字"

    var contextAfter = "宿主里光标后的字"

    func switchToNextKeyboard() {}

    var dismissCalls = 0

    func dismissKeyboard() { dismissCalls += 1 }

    var pasteboardChangeCount = 7

    var pasteboardHasText = true

    func readPasteboard() -> String? { "剪贴板" }
}

@MainActor
final class NoteComposeTests: XCTestCase {
    private func router() -> (OutputRouter, FakeOutput) {
        let host = FakeOutput()
        let router = OutputRouter()
        router.host = host
        return (router, host)
    }

    // MARK: 改道

    func testComposingNeverWritesToHost() {
        let (router, host) = router()
        router.beginNote()
        // 照 KeyboardModel 打「你好」再删一个字、拖空格、取消时清拼音的调用顺序走一遍
        router.setMarked("n")
        router.setMarked("ni")
        router.commit("你")
        router.setMarked("")
        router.setMarked("hao")
        router.commit("好")
        router.setMarked("")
        router.commit(" ")
        router.deleteBackward()
        router.deleteBackward()
        router.moveCursor(by: -3)
        router.moveCursor(by: 2)
        router.setMarked("")
        XCTAssertEqual(host.setMarkedCalls, [])
        XCTAssertEqual(host.commitCalls, [])
        XCTAssertEqual(host.deleteCalls, 0)
        XCTAssertEqual(host.moveCalls, [])
        XCTAssertEqual(router.composer?.text, "你")
    }

    func testDismissKeyboardReachesHost() {
        let (router, host) = router()
        router.dismissKeyboard()
        XCTAssertEqual(host.dismissCalls, 1, "工具栏的向下箭头收起键盘")
        XCTAssertEqual(host.writes, 0)
    }

    func testComposingContextComesFromDraftNotHost() {
        let (router, _) = router()
        router.beginNote()
        router.commit("今天")
        XCTAssertEqual(router.contextBefore, "今天")
        XCTAssertEqual(router.contextAfter, "")
    }

    func testNotComposingForwardsToHost() {
        let (router, host) = router()
        router.setMarked("ni")
        router.commit("你")
        router.deleteBackward()
        router.moveCursor(by: 1)
        XCTAssertEqual(host.setMarkedCalls, ["ni"])
        XCTAssertEqual(host.commitCalls, ["你"])
        XCTAssertEqual(host.deleteCalls, 1)
        XCTAssertEqual(host.moveCalls, [1])
        XCTAssertEqual(router.contextBefore, "宿主里光标前的字")
        XCTAssertNil(router.composer)
    }

    /// 换输入框、收起键盘、取消都走 endNote：草稿丢掉，之后的输出回到宿主。
    func testEndingDropsDraftAndRestoresHost() {
        let (router, host) = router()
        router.beginNote()
        router.commit("没记完的")
        router.endNote()
        XCTAssertNil(router.composer)
        XCTAssertFalse(router.isComposingNote)
        XCTAssertEqual(host.writes, 0)
        router.beginNote()
        XCTAssertEqual(router.composer?.text, "", "重新进手写是空草稿")
        router.endNote()
        router.commit("好")
        XCTAssertEqual(host.commitCalls, ["好"])
    }

    // MARK: 草稿

    func testAppendClampsAt200() {
        var composer = NoteComposer()
        composer.append(String(repeating: "字", count: 150))
        composer.append(String(repeating: "多", count: 80))
        XCTAssertEqual(MemoryLimits.count(composer.text), 200)
        XCTAssertTrue(composer.text.hasSuffix("多"))
        composer.append("再")
        XCTAssertEqual(MemoryLimits.count(composer.text), 200)
        XCTAssertFalse(composer.text.hasSuffix("再"))
    }

    func testAppendDropsNewlines() {
        var composer = NoteComposer()
        composer.append("一行\n两行\r\n")
        XCTAssertEqual(composer.text, "一行两行")
    }

    func testDeleteBackwardRemovesLastCharacter() {
        var composer = NoteComposer()
        composer.append("喜欢👍🏽")
        composer.deleteBackward()
        XCTAssertEqual(composer.text, "喜欢")
        composer.deleteBackward()
        composer.deleteBackward()
        composer.deleteBackward()
        XCTAssertEqual(composer.text, "")
    }

    func testEmptyOrBlankDraftCannotSave() {
        var composer = NoteComposer()
        XCTAssertFalse(composer.canSave)
        composer.append("  ")
        XCTAssertFalse(composer.canSave)
        composer.append("怕冷")
        XCTAssertTrue(composer.canSave)
    }

    func testPlaceholderUsesContactName() {
        XCTAssertEqual(NoteComposer.placeholder(name: "小美"), "记一件关于小美的事")
        XCTAssertEqual(NoteComposer.placeholder(name: "阿杰"), "记一件关于阿杰的事")
        XCTAssertEqual(NoteComposer.placeholder(name: nil), "记一件关于TA的事")
    }

    // MARK: 入口

    func testEntryGoesToComposeWhenClipboardEmptyOrHandled() {
        XCTAssertEqual(NoteEntry.decide(clipboard: nil, lastHandledDigest: nil), .compose)
        XCTAssertEqual(NoteEntry.decide(clipboard: " \n", lastHandledDigest: nil), .compose)
        let handled = NoteEntry.digest("她不吃香菜")
        XCTAssertEqual(NoteEntry.decide(clipboard: "  她不吃香菜\n", lastHandledDigest: handled), .compose)
    }

    func testEntryOffersNewClipboard() {
        let handled = NoteEntry.digest("上一条")
        XCTAssertEqual(NoteEntry.decide(clipboard: " 她不吃香菜 ", lastHandledDigest: handled), .clipboard(["她不吃香菜"]))
        let medium = String(repeating: "长", count: 250)
        XCTAssertEqual(NoteEntry.decide(clipboard: medium, lastHandledDigest: nil), .clipboard([medium]), "750 字节还是一条")
        // 一个汉字 3 字节：2000 字节装 666 个，第 667 个起是下一条
        let long = String(repeating: "长", count: 700)
        XCTAssertEqual(NoteEntry.decide(clipboard: long, lastHandledDigest: nil), .clipboard([String(repeating: "长", count: 666), String(repeating: "长", count: 34)]), "超过 2000 字节拆成两条，不丢")
    }
}
