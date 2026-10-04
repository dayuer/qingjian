// 衬线标题字：字重到宋体 PostScript 名的对应；模拟器上系统宋体在不在只打出来，不断言（真机以日志为准）。

import SwiftUI
import UIKit
import XCTest
@testable import QingjianCloud

final class SerifFontTests: XCTestCase {
    func testWeightMapsToSongtiNames() {
        XCTAssertEqual(SerifFont.postScriptName(for: .semibold), "STSongti-SC-Bold")
        XCTAssertEqual(SerifFont.postScriptName(for: .bold), "STSongti-SC-Bold")
        XCTAssertEqual(SerifFont.postScriptName(for: .medium), "STSongti-SC-Regular")
        XCTAssertEqual(SerifFont.postScriptName(for: .regular), "STSongti-SC-Regular")
    }

    func testReportsWhetherSystemHasSongti() {
        let available = UIFont(name: "STSongti-SC-Bold", size: 17) != nil
        print("系统宋体 STSongti-SC-Bold 可用：\(available)")
    }
}
