// 提醒文案里怎么称呼对象，与桥的 Pronoun 一一对应（JSON 值 ta / ta_m / ta_f / name）。

enum MemoryPronoun: String, Codable, CaseIterable, Sendable {
    case ta
    case taM = "ta_m"
    case taF = "ta_f"
    case name

    /// 选称呼时的顺序（缺省 TA）。
    static let choices: [MemoryPronoun] = [.taM, .taF, .ta, .name]

    /// 文案里的称呼，与桥的 `Pronoun::label` 一致。
    func label(name: String) -> String {
        switch self {
        case .ta: "TA"
        case .taM: "他"
        case .taF: "她"
        case .name: name
        }
    }

    /// 选称呼时的选项名。
    var title: String {
        switch self {
        case .ta: "TA"
        case .taM: "他"
        case .taF: "她"
        case .name: "直接用名字"
        }
    }
}
