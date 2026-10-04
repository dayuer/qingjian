// 主题色：oklch 换算成 sRGB 与写死的十六进制一致（容差 1/255）。

import XCTest
@testable import QingjianCloud

final class ThemeTests: XCTestCase {
    func testSwatchesMatchOKLCHSources() {
        for swatch in [Theme.accent, Theme.accentInk, Theme.accentSoft] {
            let converted = OKLCH.srgb(l: swatch.l, c: swatch.c, h: swatch.h)
            XCTAssertLessThanOrEqual(abs(converted.r - swatch.r), 1, "\(swatch.hex) r")
            XCTAssertLessThanOrEqual(abs(converted.g - swatch.g), 1, "\(swatch.hex) g")
            XCTAssertLessThanOrEqual(abs(converted.b - swatch.b), 1, "\(swatch.hex) b")
        }
    }

    func testSwatchHexValues() {
        XCTAssertEqual(Theme.accent.hex, 0xC0E7C6)
        XCTAssertEqual(Theme.accentInk.hex, 0x2E4A34)
        XCTAssertEqual(Theme.accentSoft.hex, 0xE8F9EB)
    }

    func testOKLCHKnownPoints() {
        let white = OKLCH.srgb(l: 1, c: 0, h: 0)
        XCTAssertEqual([white.r, white.g, white.b], [255, 255, 255])
        let black = OKLCH.srgb(l: 0, c: 0, h: 0)
        XCTAssertEqual([black.r, black.g, black.b], [0, 0, 0])
    }
}
