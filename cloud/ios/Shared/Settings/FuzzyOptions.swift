// 模糊音开关，与 Core 的 `FuzzyRules`（config.toml 的 [fuzzy]）同名。

import Foundation

struct FuzzyOptions: Codable, Equatable {
    var zZh: Bool

    var cCh: Bool

    var sSh: Bool

    var nL: Bool

    var fH: Bool

    var lR: Bool

    var anAng: Bool

    var enEng: Bool

    var inIng: Bool

    /// 设置页逐行列出的顺序与说明。
    static var rows: [(label: String, path: WritableKeyPath<FuzzyOptions, Bool>)] {
        [
            ("z = zh", \.zZh), ("c = ch", \.cCh), ("s = sh", \.sSh),
            ("n = l", \.nL), ("f = h", \.fH), ("l = r", \.lR),
            ("an = ang", \.anAng), ("en = eng", \.enEng), ("in = ing", \.inIng),
        ]
    }
}
