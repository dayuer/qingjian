// 改写用哪个技能怎么算：选中的人身上有就用他的 → 设置里的默认 → 列表第一个；没有技能包、没开完全访问、私密输入框里整块不出现。

import XCTest
@testable import QingjianCloud

final class RewriteSkillTests: XCTestCase {
    private func skill(_ id: String, _ name: String) -> Skill {
        Skill(id: id, name: name, summary: "")
    }

    func testRewriteNeedsSkillsAndFullAccessAndAPublicField() {
        XCTAssertTrue(
            ScopeDisplay.canRewrite(
                fullAccess: true, privateField: false, hasSkills: true, hasRewriter: true))
        XCTAssertFalse(
            ScopeDisplay.canRewrite(
                fullAccess: false, privateField: false, hasSkills: true, hasRewriter: true),
            "没开完全访问就没有网络，改写按下去必然失败")
        XCTAssertFalse(
            ScopeDisplay.canRewrite(
                fullAccess: true, privateField: true, hasSkills: true, hasRewriter: true),
            "私密输入框（密码、验证码）里不发原文")
        XCTAssertFalse(
            ScopeDisplay.canRewrite(
                fullAccess: true, privateField: false, hasSkills: false, hasRewriter: true),
            "没有技能包就整块不出现")
        XCTAssertFalse(
            ScopeDisplay.canRewrite(
                fullAccess: true, privateField: false, hasSkills: true, hasRewriter: false),
            "没配云服务就没有改写器")
    }

    func testMissingSummaryDecodesAsEmpty() throws {
        let json = #"[{"id":"polish","name":"润色"},{"id":"tactful","name":"高情商","summary":"说得得体"}]"#
        let skills = try JSONDecoder().decode([Skill].self, from: Data(json.utf8))
        XCTAssertEqual(skills.map(\.id), ["polish", "tactful"])
        XCTAssertEqual(skills[0].summary, "")
        XCTAssertEqual(skills[1].summary, "说得得体")
    }

    func testTheContactSkillWins() {
        let skills = [skill("polish", "润色"), skill("tactful", "高情商")]
        XCTAssertEqual(
            RewriteSkill.resolve(skills: skills, contactSkill: "tactful", defaultSkill: "polish")?
                .id,
            "tactful")
    }

    func testFallsBackToTheDefaultThenTheFirst() {
        let skills = [skill("polish", "润色"), skill("tactful", "高情商")]
        XCTAssertEqual(
            RewriteSkill.resolve(skills: skills, contactSkill: nil, defaultSkill: "tactful")?.id,
            "tactful")
        XCTAssertEqual(
            RewriteSkill.resolve(skills: skills, contactSkill: "nope", defaultSkill: "nope")?.id,
            "polish", "两个都认不得时用列表第一个")
        XCTAssertNil(RewriteSkill.resolve(skills: [], contactSkill: nil, defaultSkill: "polish"))
    }

    func testTheRowOffersTheDefaultFirst() {
        let rows = RewriteSkill.options(
            skills: [skill("polish", "润色"), skill("tactful", "高情商")], current: "tactful")
        XCTAssertEqual(rows.map(\.title), ["用默认", "润色", "高情商"])
        XCTAssertEqual(rows.map(\.selected), [false, false, true])
        XCTAssertEqual(rows.map(\.id), [nil, "polish", "tactful"])
        XCTAssertEqual(
            RewriteSkill.options(skills: [], current: nil).map(\.selected), [true],
            "没选人（身上没指定）时「用默认」是选中那个")
    }

    // MARK: 设置里的全局默认技能（键盘读过桥的 qj_rewrite_default）

    func testTheGlobalDefaultDecodesTheBridgeJSON() {
        XCTAssertEqual(
            RewriteDefault.decode(#"{"skill":"tactful"}"#), RewriteDefault(skill: "tactful"))
        XCTAssertEqual(
            RewriteDefault.decode(#"{"skill":"tactful","extra":1}"#), RewriteDefault(skill: "tactful"),
            "多出来的字段忽略")
    }

    func testTheGlobalDefaultFallsBackWhenTheBridgeGivesNothing() {
        // 读不是用户刚做的动作：桥返回 NULL（没有配置文件）或给的不是那个 JSON 时按缺省，不弹错
        XCTAssertEqual(RewriteDefault.decode(nil), RewriteDefault(skill: "polish"))
        XCTAssertEqual(RewriteDefault.decode("not json"), RewriteDefault(skill: "polish"))
        XCTAssertEqual(RewriteDefault.decode(#"{"other":1}"#), RewriteDefault(skill: "polish"))
    }
}
