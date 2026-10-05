// 守护：键盘画出来的文字、VoiceOver 读的、日志里写的称呼只能是 chipName，不能出现真名。
// 做法是读 Keyboard/Sources 的源码，任何对 `.name` 属性的读取都算违规（`ContactAdd.name(…)` 这类函数调用不算）；
// 技能名（技能包的 `Skill.name`，不是人的名字）放行；上一版还放行过场景名，场景去掉之后那条已无命中，一并删了。
// 源码按 #filePath 找，只在模拟器上跑（模拟器与 Mac 共用文件系统）。

import Foundation
import XCTest

final class KeyboardNameGuardTests: XCTestCase {
    func testKeyboardSourcesNeverReadContactName() throws {
        let sources = URL(fileURLWithPath: #filePath)
            .deletingLastPathComponent()
            .deletingLastPathComponent()
            .appendingPathComponent("Keyboard/Sources")
        let files = try FileManager.default.contentsOfDirectory(at: sources, includingPropertiesForKeys: nil)
            .filter { $0.pathExtension == "swift" }
        XCTAssertGreaterThan(files.count, 10, "没找到键盘源码：\(sources.path)")
        let pattern = try NSRegularExpression(pattern: #"\.name\b(?!\s*\()"#)
        // 白名单：技能名（`Skill.name`）与拿到它之后的 `.name` 不算称呼
        let allowed = try NSRegularExpression(pattern: #"(skill|Skill|\?)\.name\b"#)
        var hits: [String] = []
        for file in files {
            let text = try String(contentsOf: file, encoding: .utf8)
            for (number, line) in text.components(separatedBy: "\n").enumerated() {
                let range = NSRange(line.startIndex..., in: line)
                if pattern.firstMatch(in: line, range: range) != nil,
                   allowed.firstMatch(in: line, range: range) == nil {
                    hits.append("\(file.lastPathComponent):\(number + 1): \(line.trimmingCharacters(in: .whitespaces))")
                }
            }
        }
        XCTAssertEqual(hits, [], "键盘里的称呼要用 chipName：\n" + hits.joined(separator: "\n"))
    }
}
