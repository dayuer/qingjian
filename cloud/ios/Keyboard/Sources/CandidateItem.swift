// 候选栏里的一格：文字，以及是不是大模型给的（用强调色画）。

struct CandidateItem: Hashable {
    let text: String

    let cloud: Bool
}
