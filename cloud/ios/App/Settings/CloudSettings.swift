// 青简 Cloud 状态与开关，与桥的 `CloudStatus` 对应。服务器地址只读，令牌不经过设置页。

import Foundation

struct CloudSettings: Codable, Equatable {
    var server: String

    /// 地址与令牌都配好了，键盘会连。
    var connected: Bool

    var llm: Bool

    var candidates: Bool

    var logs: Bool

    var sync: Bool

    var clipboard: Bool

    /// 写回时只带开关（桥按 `CloudSwitches` 解析，地址与令牌不动）。
    var switches: [String: Bool] {
        ["llm": llm, "candidates": candidates, "logs": logs, "sync": sync, "clipboard": clipboard]
    }
}
