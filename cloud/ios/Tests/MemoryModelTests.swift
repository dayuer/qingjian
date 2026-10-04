// 键盘与 App 共用的记忆模型：桥 JSON 解码、上限、日子换算、提示行与牌子的显示判断、换输入框判断。

import XCTest
@testable import QingjianCloud

final class MemoryModelTests: XCTestCase {
    private func contact(_ name: String = "小美") -> MemoryContact {
        MemoryContact.new(name: name, pronoun: .taF)
    }

    private func date(_ text: String) throws -> Date { try XCTUnwrap(MemoryDate.parse(text)) }

    // MARK: 上限

    func testLimitsCountUnicodeScalars() {
        XCTAssertEqual(MemoryLimits.maxTextChars, 200)
        XCTAssertEqual(MemoryLimits.maxKeywords, 8)
        XCTAssertEqual(MemoryLimits.count("不吃香菜"), 4)
        XCTAssertEqual(MemoryLimits.count("🎂"), 1)
    }

    func testClampTextCutsAt200Scalars() {
        let long = String(repeating: "字", count: 250)
        XCTAssertEqual(MemoryLimits.count(MemoryLimits.clampText(long)), 200)
        let short = "不吃香菜"
        XCTAssertEqual(MemoryLimits.clampText(short), short)
        XCTAssertEqual(MemoryLimits.counter("不吃香菜"), "4 / 200")
    }

    func testKeywordRules() {
        XCTAssertTrue(MemoryLimits.canAdd("蛋糕", to: []))
        XCTAssertTrue(MemoryLimits.canAdd(" 蛋糕 ", to: []), "首尾空白不计字数")
        XCTAssertFalse(MemoryLimits.canAdd("糕", to: []), "少于 2 字")
        XCTAssertTrue(MemoryLimits.canAdd("草莓味的奶油蛋糕", to: []), "8 字可以")
        XCTAssertFalse(MemoryLimits.canAdd("草莓味的奶油蛋糕卷", to: []), "多于 8 字")
        XCTAssertFalse(MemoryLimits.canAdd("蛋糕", to: ["蛋糕"]), "重复")
        let full = (0..<8).map { "关键词\($0)" }
        XCTAssertFalse(MemoryLimits.canAdd("新词语", to: full), "已满 8 个")
    }

    // MARK: 桥的 JSON

    func testContactDecodesWithDefaultsForOldFiles() throws {
        let json = #"{"id":"0123456789abcdef0123456789abcdef","name":"小美","pronoun":"ta_f","scene":"dating","created_at":100}"#
        let contact = try JSONDecoder().decode(MemoryContact.self, from: Data(json.utf8))
        XCTAssertEqual(contact.name, "小美")
        XCTAssertEqual(contact.pronoun, .taF)
        XCTAssertTrue(contact.hintOn)
        XCTAssertTrue(contact.remindOn)
        let off = #"{"id":"a","name":"b","scene":"dating","created_at":1,"hint_on":false,"remind_on":false}"#
        let decoded = try JSONDecoder().decode(MemoryContact.self, from: Data(off.utf8))
        XCTAssertFalse(decoded.hintOn)
        XCTAssertFalse(decoded.remindOn)
        XCTAssertEqual(decoded.pronoun, .ta, "缺称呼按 TA")
    }

    func testCardKeepsCloudFieldsOnRoundTrip() throws {
        let json = #"{"id":"a","kind":"date","text":"生日","keywords":[],"when":"1998-05-20","source":"cloud","confirmed":true,"faded":true,"seq":7,"updated_at":1700000000000,"created_at":1,"touched_at":2}"#
        let card = try JSONDecoder().decode(MemoryCard.self, from: Data(json.utf8))
        XCTAssertTrue(card.faded)
        XCTAssertEqual(card.seq, 7)
        XCTAssertEqual(card.updatedAt, 1_700_000_000_000)
        let back = try JSONDecoder().decode(MemoryCard.self, from: JSONEncoder().encode(card))
        XCTAssertEqual(back, card)
    }

    func testCardDecodesWithoutOptionalFields() throws {
        let json = #"{"id":"a","kind":"preference","text":"不吃香菜","created_at":1,"touched_at":2}"#
        let card = try JSONDecoder().decode(MemoryCard.self, from: Data(json.utf8))
        XCTAssertEqual(card.keywords, [])
        XCTAssertNil(card.when)
        XCTAssertEqual(card.source, "manual")
        XCTAssertFalse(card.faded)
        XCTAssertEqual(card.seq, 0)
    }

    func testSnapshotDecodesFromBridge() throws {
        let json = """
        {"contacts":[{"id":"a","name":"小美","pronoun":"ta_f","scene":"dating","created_at":1}],
         "cards":{"a":[{"id":"c","kind":"other","text":"x","created_at":1,"touched_at":1}]},
         "revs":{"a":3},"state":{"scene":"dating","contact_id":"a"},"broken":["b"]}
        """
        let snapshot = try JSONDecoder().decode(MemorySnapshot.self, from: Data(json.utf8))
        XCTAssertEqual(snapshot.contacts.count, 1)
        XCTAssertEqual(snapshot.cards["a"]?.count, 1)
        XCTAssertEqual(snapshot.revs["a"], 3)
        XCTAssertEqual(snapshot.state, MemoryScope(scene: "dating", contactId: "a"))
        XCTAssertEqual(snapshot.broken, ["b"])
        let empty = try JSONDecoder().decode(MemorySnapshot.self, from: Data("{}".utf8))
        XCTAssertEqual(empty, MemorySnapshot())
    }

    func testScopeDecodes() throws {
        let scope = try JSONDecoder().decode(MemoryScope.self, from: Data(#"{"scene":"work","contact_id":null}"#.utf8))
        XCTAssertEqual(scope, MemoryScope(scene: "work", contactId: nil))
        XCTAssertEqual(MemoryScope.title(of: "dating"), "恋爱")
        XCTAssertEqual(MemoryScope.title(of: "work"), "工作")
        XCTAssertEqual(MemoryScope.title(of: "daily"), "日常")
    }

    func testHintDecodes() throws {
        let json = #"{"card_id":"c","text":"明天是她的生日","reason":"today","more":true}"#
        let hint = try JSONDecoder().decode(MemoryHint.self, from: Data(json.utf8))
        XCTAssertEqual(hint, MemoryHint(cardId: "c", text: "明天是她的生日", reason: .today, more: true))
    }

    // MARK: qj_memory_note 的返回值：NULL 就是成功

    func testNoteNullMeansSuccess() {
        XCTAssertNil(MemoryFailure.decode(nil), "桥返回 NULL（含已接受、稍后写入）：成功")
    }

    func testNoteFailureDecodes() {
        let failure = MemoryFailure.decode(#"{"code":"io","message":"读不了"}"#)
        XCTAssertEqual(failure, MemoryFailure(code: .io, message: "读不了"))
        XCTAssertEqual(MemoryFailure.decode(#"{"code":"contact_limit","message":"x"}"#)?.userMessage, "恋爱场景最多 8 个人")
        XCTAssertEqual(MemoryFailure.decode(#"{"code":"zzz","message":"m"}"#)?.code, .other)
        XCTAssertEqual(MemoryFailure.decode("not json")?.message, "not json")
    }

    // MARK: 日子

    func testDaysAwayRepeatsDatesYearlyAndPromisesOnce() throws {
        let now = try date("2026-10-04")
        var card = MemoryCard.new(kind: .date, text: "生日", when: "1998-10-05", keywords: [])
        XCTAssertEqual(card.daysAway(now: now), 1)
        card.when = "1998-10-04"
        XCTAssertEqual(card.daysAway(now: now), 0)
        card.when = "1998-10-03"
        XCTAssertEqual(card.daysAway(now: now), 364, "今年那天过了看明年")
        card.kind = .promise
        card.when = "2026-10-03"
        XCTAssertEqual(card.daysAway(now: now), -1, "约定不重复")
        card.kind = .preference
        XCTAssertNil(card.daysAway(now: now))
    }

    func testLeapDayFallsOnFeb28InCommonYears() throws {
        let next = MemoryDate.nextAnniversary(of: try date("2000-02-29"), from: try date("2026-10-04"))
        XCTAssertEqual(MemoryDate.format(next), "2027-02-28")
        let leap = MemoryDate.nextAnniversary(of: try date("2000-02-29"), from: try date("2027-10-04"))
        XCTAssertEqual(MemoryDate.format(leap), "2028-02-29")
    }

    func testReminderTextMatchesBridgeTemplates() {
        let person = contact()
        let birthday = MemoryCard.new(kind: .date, text: "生日", when: "1998-10-05", keywords: [])
        XCTAssertEqual(birthday.reminderText(days: 1, contact: person), "明天是她的生日")
        XCTAssertEqual(birthday.reminderText(days: 0, contact: person), "今天是她的生日")
        XCTAssertEqual(birthday.reminderText(days: 3, contact: person), "3 天后是她的生日")
        let promise = MemoryCard.new(kind: .promise, text: "看电影", when: "2026-10-05", keywords: [])
        XCTAssertEqual(promise.reminderText(days: 1, contact: person), "明天：看电影")
    }

    func testKnownDaysCountsCreationDayAsFirst() {
        var person = contact()
        XCTAssertEqual(person.knownDays(), 1)
        person = MemoryContact(
            id: "a", name: "小美", pronoun: .ta, scene: "dating",
            createdAt: Int64(Date().timeIntervalSince1970) - 12 * 86400)
        XCTAssertEqual(person.knownDays(), 13)
    }

    func testIDFormatIs32LowercaseHex() {
        let id = MemoryID.make()
        XCTAssertEqual(id.count, 32)
        XCTAssertTrue(id.allSatisfy { $0.isHexDigit && !$0.isUppercase })
        XCTAssertNotEqual(id, MemoryID.make())
    }

    // MARK: 提示行、牌子、面板

    func testHintRowOnlyForDatingWithContact() {
        XCTAssertTrue(ScopeDisplay.hasHintRow(scene: "dating", hasContact: true, hasContent: true))
        XCTAssertFalse(
            ScopeDisplay.hasHintRow(scene: "dating", hasContact: true, hasContent: false), "没有提示就不占行，不留空行")
        XCTAssertFalse(ScopeDisplay.hasHintRow(scene: "dating", hasContact: false, hasContent: true), "恋爱不指定没有提示行")
        XCTAssertFalse(ScopeDisplay.hasHintRow(scene: "daily", hasContact: true, hasContent: true))
        XCTAssertFalse(ScopeDisplay.hasHintRow(scene: "work", hasContact: true, hasContent: true))
    }

    func testChipTitle() {
        XCTAssertEqual(ScopeDisplay.chipTitle(scene: "dating", contactName: "小美"), "小美 · 恋爱")
        XCTAssertEqual(ScopeDisplay.chipTitle(scene: "dating", contactName: nil), "恋爱")
        XCTAssertEqual(ScopeDisplay.chipTitle(scene: "work", contactName: "小美"), "工作")
        XCTAssertEqual(ScopeDisplay.chipTitle(scene: "daily", contactName: nil), "日常")
    }

    func testPickerNeedsFullAccess() {
        XCTAssertEqual(ScopeDisplay.pickerMode(fullAccess: false), .needsFullAccess)
        XCTAssertEqual(ScopeDisplay.pickerMode(fullAccess: true), .picker)
        XCTAssertEqual(ScopeDisplay.needsFullAccessText, "开启完全访问后才能使用记忆")
    }

    func testCanNoteNeedsEverything() {
        XCTAssertTrue(ScopeDisplay.canNote(fullAccess: true, clipboardHasText: true, privateField: false, hasContact: true))
        XCTAssertFalse(ScopeDisplay.canNote(fullAccess: false, clipboardHasText: true, privateField: false, hasContact: true))
        XCTAssertFalse(ScopeDisplay.canNote(fullAccess: true, clipboardHasText: false, privateField: false, hasContact: true))
        XCTAssertFalse(ScopeDisplay.canNote(fullAccess: true, clipboardHasText: true, privateField: true, hasContact: true))
        XCTAssertFalse(ScopeDisplay.canNote(fullAccess: true, clipboardHasText: true, privateField: false, hasContact: false))
    }

    // MARK: 换输入框

    func testHostChangedComparesDocumentIdentifier() {
        let a = UUID()
        let b = UUID()
        XCTAssertFalse(HostDocument.changed(from: a, to: a))
        XCTAssertTrue(HostDocument.changed(from: a, to: b))
        XCTAssertTrue(HostDocument.changed(from: nil, to: a), "第一次看到输入框也清一次")
        XCTAssertFalse(HostDocument.changed(from: nil, to: nil))
        XCTAssertTrue(HostDocument.changed(from: a, to: nil), "离开输入框（标识变 nil）也算换了，组字要清")
    }
}
