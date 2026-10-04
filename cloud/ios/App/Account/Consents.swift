// 四项云功能开没开，与协议的 `Consents` 对应。

import Foundation

struct Consents: Codable, Equatable {
    var clipboard: Bool

    var sync: Bool

    var inputLog: Bool

    var llm: Bool

    subscript(feature: CloudFeature) -> Bool {
        get {
            switch feature {
            case .clipboard: clipboard
            case .sync: sync
            case .inputLog: inputLog
            case .llm: llm
            }
        }
        set {
            switch feature {
            case .clipboard: clipboard = newValue
            case .sync: sync = newValue
            case .inputLog: inputLog = newValue
            case .llm: llm = newValue
            }
        }
    }
}
