// 主题色：浅 / 深两套 oklch 换算成 sRGB 与写死的十六进制一致（容差 1/255）。

import XCTest
@testable import QingjianCloud

final class ThemeTests: XCTestCase {
    private var swatches: [ThemeSwatch] {
        [Theme.accent, Theme.accentInk, Theme.accentSoft, Theme.hintLine, Theme.ink2Tone,
         Theme.paperTone, Theme.paperCardTone].flatMap { [$0.light, $0.dark] }
    }

    func testSwatchesMatchOKLCHSources() {
        for swatch in swatches {
            let converted = OKLCH.srgb(l: swatch.l, c: swatch.c, h: swatch.h)
            XCTAssertLessThanOrEqual(abs(converted.r - swatch.r), 1, "\(swatch.hex) r")
            XCTAssertLessThanOrEqual(abs(converted.g - swatch.g), 1, "\(swatch.hex) g")
            XCTAssertLessThanOrEqual(abs(converted.b - swatch.b), 1, "\(swatch.hex) b")
        }
    }

    func testPaperHexValues() {
        XCTAssertEqual(Theme.paperTone.light.hex, 0xFCFCFC)
        XCTAssertEqual(Theme.paperTone.dark.hex, 0x161616)
        XCTAssertEqual(Theme.paperCardTone.light.hex, 0xFFFFFF)
        XCTAssertEqual(Theme.paperCardTone.dark.hex, 0x1F1F1F)
    }

    func testLightHexValues() {
        XCTAssertEqual(Theme.accent.light.hex, 0xC0E7C6)
        XCTAssertEqual(Theme.accentInk.light.hex, 0x2E4A34)
        XCTAssertEqual(Theme.accentSoft.light.hex, 0xE8F9EB)
    }

    func testDarkHexValues() {
        XCTAssertEqual(Theme.accent.dark.hex, 0x34563B)
        XCTAssertEqual(Theme.accentInk.dark.hex, 0xB6DDBD)
        XCTAssertEqual(Theme.accentSoft.dark.hex, 0x1D271F)
    }

    func testDarkSourcesAreTheDesignValues() {
        XCTAssertEqual([Theme.accent.dark.l, Theme.accent.dark.c, Theme.accent.dark.h], [0.42, 0.06, 150])
        XCTAssertEqual([Theme.accentInk.dark.l, Theme.accentInk.dark.c, Theme.accentInk.dark.h], [0.86, 0.06, 150])
        XCTAssertEqual([Theme.accentSoft.dark.l, Theme.accentSoft.dark.c, Theme.accentSoft.dark.h], [0.26, 0.02, 150])
    }

    func testDynamicColorFollowsInterfaceStyle() {
        let color = Theme.accent.uiColor
        let light = color.resolvedColor(with: UITraitCollection(userInterfaceStyle: .light))
        let dark = color.resolvedColor(with: UITraitCollection(userInterfaceStyle: .dark))
        XCTAssertEqual(light, Theme.accent.light.uiColor)
        XCTAssertEqual(dark, Theme.accent.dark.uiColor)
    }

    func testOKLCHKnownPoints() {
        let white = OKLCH.srgb(l: 1, c: 0, h: 0)
        XCTAssertEqual([white.r, white.g, white.b], [255, 255, 255])
        let black = OKLCH.srgb(l: 0, c: 0, h: 0)
        XCTAssertEqual([black.r, black.g, black.b], [0, 0, 0])
    }
}
