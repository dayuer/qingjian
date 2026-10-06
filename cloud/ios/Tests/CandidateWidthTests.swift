// 候选格自己算的宽要与原先 Auto Layout 量出的一样（标签的 intrinsicContentSize 加左右留白），样式才不会悄悄变。
// 覆盖中文、emoji、英文、长整句、单字（撞最小宽）、首选的加粗字重。

import UIKit
import XCTest
@testable import QingjianCloud

final class CandidateWidthTests: XCTestCase {
    private let samples = [
        "哈哈", "😃", "🙏", "hello", "helloWorld", "HelloSign", "OK", "我想去看看", "i", "一",
        "我们一起去看演出吧然后去吃饭", "👨‍👩‍👧‍👦", "Wi-Fi",
    ]

    /// 原先的量法：标签按字体自然宽度，格子再加左右留白，至少 minimum。
    /// 不在窗口里的标签按主屏的 scale 取整。
    private func labelWidth(_ text: String, highlighted: Bool) -> CGFloat {
        let label = UILabel()
        label.font = CandidateWidth.font(highlighted: highlighted)
        label.text = text
        return max(label.intrinsicContentSize.width + CandidateWidth.padding * 2, CandidateWidth.minimum)
    }

    func testMeasureMatchesTheLabel() {
        let scale = UIScreen.main.scale
        for text in samples {
            for highlighted in [false, true] {
                XCTAssertEqual(
                    CandidateWidth.measure(text, highlighted: highlighted, scale: scale),
                    labelWidth(text, highlighted: highlighted), accuracy: 0.001,
                    "\(text) highlighted=\(highlighted)")
            }
        }
    }

    func testShortTextHitsTheMinimum() {
        XCTAssertEqual(CandidateWidth.measure("i", highlighted: false, scale: 3), CandidateWidth.minimum)
    }

    func testHighlightedIsMeasuredWithItsOwnWeight() {
        let regular = CandidateWidth.measure("helloWorld", highlighted: false, scale: 3)
        let medium = CandidateWidth.measure("helloWorld", highlighted: true, scale: 3)
        XCTAssertGreaterThan(medium, regular)
    }

    func testCacheCapsAtTheLimitAndKeysByHighlight() {
        let cache = CandidateWidthCache()
        let long = "我们一起去看演出吧然后去吃饭再去散步"
        XCTAssertEqual(cache.width(long, highlighted: false, scale: 3, limit: 200), 200)
        XCTAssertEqual(cache.width("哈哈", highlighted: false, scale: 3, limit: 200),
                       CandidateWidth.measure("哈哈", highlighted: false, scale: 3))
        _ = cache.width("哈哈", highlighted: true, scale: 3, limit: 200)
        _ = cache.width("哈哈", highlighted: false, scale: 3, limit: 200)
        XCTAssertEqual(cache.count, 3)
    }

    func testCacheEmptiesWhenFull() {
        let cache = CandidateWidthCache(capacity: 2)
        _ = cache.width("一", highlighted: false, scale: 3, limit: 300)
        _ = cache.width("二", highlighted: false, scale: 3, limit: 300)
        _ = cache.width("三", highlighted: false, scale: 3, limit: 300)
        XCTAssertEqual(cache.count, 1)
    }
}
