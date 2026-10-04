// 要不要出首次引导：没看过、并且读到的对象为空才出。看没看过记在 App 自己的 UserDefaults（键盘不需要知道，不进 App Group）。
// 读失败或 App Group 不可用一律当作「没有对象」：宁可多出一次引导，也不能因为读不到就让新用户错过它。

enum OnboardingGate {
    /// UserDefaults.standard 里的键：true 表示看过（走完或跳过）。
    static let doneKey = "onboardingDone"

    /// `contacts` 为 nil 表示没读出来（容器不可用、读取失败）。
    static func shouldShow(done: Bool, contacts: [MemoryContact]?) -> Bool {
        !done && (contacts ?? []).isEmpty
    }
}
