// 提示行文字：关键词加粗、来源标签；对象卡与选择面板的文字（相对日子、副文字、页脚）。

import XCTest
@testable import QingjianCloud

final class HintTextTests: XCTestCase {
    private func boldRuns(_ attributed: AttributedString) -> [String] {
        attributed.runs.compactMap { run in
            run.inlinePresentationIntent?.contains(.stronglyEmphasized) == true
                ? String(attributed[run.range].characters) : nil
        }
    }

    private func date(_ text: String) throws -> Date { try XCTUnwrap(MemoryDate.parse(text)) }

    func testBoldsFirstMatchingWord() {
        let result = HintText.attributed(text: "不吃香菜，喜欢草莓蛋糕", emphasis: ["蛋糕", "香菜"])
        XCTAssertEqual(boldRuns(result), ["蛋糕"])
        XCTAssertEqual(String(result.characters), "不吃香菜，喜欢草莓蛋糕")
    }

    func testSkipsWordsThatDoNotAppear() {
        let result = HintText.attributed(text: "明天是她的生日", emphasis: ["考试", "生日"])
        XCTAssertEqual(boldRuns(result), ["生日"])
    }

    func testNoMatchMeansNoBold() {
        XCTAssertEqual(boldRuns(HintText.attributed(text: "明天是她的生日", emphasis: ["蛋糕"])), [])
        XCTAssertEqual(boldRuns(HintText.attributed(text: "明天是她的生日", emphasis: [])), [])
        XCTAssertEqual(boldRuns(HintText.attributed(text: "明天是她的生日", emphasis: [""])), [])
    }

    func testEmojiDoesNotShiftRange() {
        let result = HintText.attributed(text: "🎂她周三考科目二👩‍❤️‍👨加油", emphasis: ["科目二"])
        XCTAssertEqual(boldRuns(result), ["科目二"])
        XCTAssertEqual(String(result.characters), "🎂她周三考科目二👩‍❤️‍👨加油")
    }

    func testEmphasisForReasons() {
        var card = MemoryCard.new(kind: .date, text: "生日", when: "1998-10-05", keywords: ["生日", "蛋糕"])
        let today = MemoryHint(cardId: "c", text: "明天是她的生日", reason: .today, more: false)
        XCTAssertEqual(HintText.emphasis(for: today, card: card), ["生日"], "日子提醒加粗卡片文字里的那个词")
        let match = MemoryHint(cardId: "c", text: "x", reason: .match, more: false)
        XCTAssertEqual(HintText.emphasis(for: match, card: card), ["生日", "蛋糕"], "匹配提示加粗关键词")
        card.keywords = []
        XCTAssertEqual(HintText.emphasis(for: match, card: card), [])
        XCTAssertEqual(HintText.emphasis(for: match, card: nil), [])
    }

    func testSourceLabel() {
        XCTAssertEqual(HintText.sourceLabel(for: "manual"), "你写的")
        XCTAssertNil(HintText.sourceLabel(for: "cloud"))
        XCTAssertEqual(HintText.sourceLabel(for: nil), "你写的", "2A 里卡片都是手动卡")
    }

    func testDateLabelIsRelativeWithinThreeDays() throws {
        let now = try date("2026-10-04")
        func label(_ when: String, kind: MemoryCard.Kind = .promise) -> String? {
            MemoryCard.new(kind: kind, text: "x", when: when, keywords: []).dateLabel(now: now)
        }
        XCTAssertEqual(label("2026-10-04"), "今天")
        XCTAssertEqual(label("2026-10-05"), "明天")
        XCTAssertEqual(label("2026-10-06"), "周二")
        XCTAssertEqual(label("2026-10-07"), "周三")
        XCTAssertEqual(label("2026-10-20"), "10.20")
        XCTAssertEqual(label("2026-12-05"), "12.05")
        XCTAssertEqual(label("1998-10-05", kind: .date), "明天", "日子按年重复，取下一次")
        XCTAssertEqual(label("1998-03-09", kind: .date), "3.09")
        XCTAssertNil(MemoryCard.new(kind: .preference, text: "x", when: nil, keywords: []).dateLabel(now: now))
    }

    func testSubtitleSkipsKeywordsEqualToTitle() {
        var card = MemoryCard.new(kind: .date, text: "生日", when: nil, keywords: ["生日"])
        XCTAssertEqual(card.subtitle, "")
        card.keywords = ["生日", "蛋糕", "惊喜"]
        XCTAssertEqual(card.subtitle, "蛋糕 · 惊喜")
        card.keywords = []
        XCTAssertEqual(card.subtitle, "")
    }

    func testPanelTexts() {
        XCTAssertEqual(ScopeDisplay.cardFooter(count: 2), "只显示与今天有关的 2 条")
        XCTAssertEqual(ScopeDisplay.contactSubtitle(knownDays: 13), "认识 13 天")
        XCTAssertEqual(ScopeDisplay.newContactSubtitle(count: 3), "3 / 8")
        XCTAssertEqual(ScopeDisplay.maxContacts, 8)
        XCTAssertEqual(ScopeDisplay.noScopeSubtitle, "只用场景")
        XCTAssertEqual(ScopeDisplay.allMemoryNotice, "在素笺 App 里查看全部记忆")
        XCTAssertEqual(ScopeDisplay.enableFullAccessNotice, "在素笺 App 里按引导开启完全访问")
    }
}
