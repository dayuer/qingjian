// MaterialsStore 弹给用户的固定文案：读不出来常驻在「待整理」一节里，删失败以「没删掉：」开头再接原因。

extension MaterialsStore {
    enum Wording {
        static let unreadable = "待整理的原话暂时读不出来（手机刚开机还没解锁过，或键盘正在写），稍后点「再读一次」"

        static func deleteFailed(_ failure: MemoryFailure) -> String { "没删掉：\(failure.userMessage)" }
    }
}
