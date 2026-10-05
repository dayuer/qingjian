// 守护：主 App 的源码不再直接用系统字体或衬线（`.font(.system(` 与 `design: .serif`），一律经 AppFont 取 MiSans。
// 字体入口 AppFont.swift 自己的回退除外；键盘（Keyboard/Sources）照旧用系统字体，不在检查范围。源码按 #filePath 找，只在模拟器上跑。

import Foundation
import XCTest

final class AppFontGuardTests: XCTestCase {
    func testAppSourcesUseAppFont() throws {
        let app = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .appendingPathComponent("App")
        let files = try XCTUnwrap(FileManager.default.enumerator(at: app, includingPropertiesForKeys: nil))
            .compactMap { $0 as? URL }
            .filter { $0.pathExtension == "swift" && $0.lastPathComponent != "AppFont.swift" }
        XCTAssertGreaterThan(files.count, 30, "没找到主 App 源码：\(app.path)")
        var hits: [String] = []
        for file in files {
            let text = try String(contentsOf: file, encoding: .utf8)
            for (number, line) in text.components(separatedBy: "\n").enumerated()
            where line.contains(".font(.system(") || line.contains("design: .serif") || line.contains("SerifFont") {
                hits.append("\(file.lastPathComponent):\(number + 1): \(line.trimmingCharacters(in: .whitespaces))")
            }
        }
        XCTAssertEqual(hits, [], "主 App 的字体要经 AppFont：\n" + hits.joined(separator: "\n"))
    }
}
