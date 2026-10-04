// 首次引导的四步与顺序：介绍（04 的 1a）、开启键盘（05 的 2d）、免费版与云服务（05 的 2e）、第一个对象（05 的 2g，复用 ContactEditor）。
// 顶部进度条四格，第 n 步亮 n 格；介绍那一步照设计稿 1a 不画进度条。

enum OnboardingStep: Int, CaseIterable {
    case intro = 1

    case keyboard

    case plan

    case contact

    /// 第几步，从 1 数。
    var number: Int { rawValue }

    var next: OnboardingStep? { OnboardingStep(rawValue: rawValue + 1) }

    var showsProgress: Bool { self != .intro }

    /// 进度条每一格亮不亮（设计稿 .progress i.on）。
    var progressCells: [Bool] { OnboardingStep.allCases.map { $0.number <= number } }
}
