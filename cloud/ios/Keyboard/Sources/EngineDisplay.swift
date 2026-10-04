// 换引擎后键盘上还该显示什么：没有引擎（数据缺失或打不开）时不留旧引擎的拼音与候选。

enum EngineDisplay {
    static func afterReplace(
        hasEngine: Bool, preedit: String, candidates: [CandidateItem]
    ) -> (preedit: String, candidates: [CandidateItem]) {
        hasEngine ? (preedit, candidates) : ("", [])
    }
}
