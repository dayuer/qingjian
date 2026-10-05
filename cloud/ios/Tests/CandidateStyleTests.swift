// 候选那一格的底色、字色、字重：首选/非首选 × accent/非 × cloud/非 全组合。
// 候选栏与展开面板共用这一套，锁住它两边才不会各走各的。

import XCTest
@testable import QingjianCloud

final class CandidateStyleTests: XCTestCase {
    func testOnlyTheFirstCandidateHasABackground() {
        XCTAssertNotNil(CandidateStyle.background(highlighted: true))
        XCTAssertNil(CandidateStyle.background(highlighted: false))
    }

    func testOnlyTheFirstCandidateIsEmphasized() {
        XCTAssertEqual(CandidateStyle.weight(highlighted: true), .medium)
        XCTAssertEqual(CandidateStyle.weight(highlighted: false), .regular)
    }

    /// 首选且 accent（恋爱、日常选了人）用强调色。
    func testAccentedFirstCandidate() {
        XCTAssertEqual(
            CandidateStyle.role(highlighted: true, accent: true, cloud: false), .accent)
        XCTAssertEqual(
            CandidateStyle.role(highlighted: true, accent: true, cloud: true), .accent,
            "首选是云候选时，强调色仍然优先")
    }

    /// 工作场景、「不指定」时 accent 为假，首选只加粗、不上强调色。
    func testNonAccentedFirstCandidateKeepsTheNormalColor() {
        XCTAssertEqual(
            CandidateStyle.role(highlighted: true, accent: false, cloud: false), .ink)
    }

    /// 大模型给的候选次一级，不管它是不是首选。
    func testCloudCandidatesAreSecondary() {
        XCTAssertEqual(
            CandidateStyle.role(highlighted: false, accent: false, cloud: true), .ink2)
        XCTAssertEqual(
            CandidateStyle.role(highlighted: true, accent: false, cloud: true), .ink2)
    }

    /// accent 只在首选时才作数：非首选传了 accent 也照云候选/正文色走。
    func testAccentOnlyCountsOnTheFirstCandidate() {
        XCTAssertEqual(
            CandidateStyle.role(highlighted: false, accent: true, cloud: false), .ink)
        XCTAssertEqual(
            CandidateStyle.role(highlighted: false, accent: true, cloud: true), .ink2)
    }

    func testPlainCandidateIsTheBodyColor() {
        XCTAssertEqual(
            CandidateStyle.role(highlighted: false, accent: false, cloud: false), .ink)
    }
}
