// 「记录中 / 已暂停」标记与暂停提示条：桥给的四种状态怎么显示、什么时候不出，以及几处文案。
// 「记录中」= 输入日志在记、在传（不是记一笔那种用户主动存的）。

enum RecordingDisplay {
    /// 工具栏牌子后面那个标记。
    enum Badge: Equatable {
        case recording
        case paused

        var title: String {
            switch self {
            case .recording: "记录中"
            case .paused: "已暂停"
            }
        }
    }

    /// 点一下暂停多久（秒）之后自动恢复。
    static let pauseSeconds: Int64 = 3600

    static let pausedBanner = "已暂停记录，提示照常。1 小时后恢复"

    static let pauseForever = "一直暂停"

    static let pausedForeverBanner = "已暂停记录，点「已暂停」恢复"

    static let resumedBanner = "已恢复记录"

    /// 桥给的状态（0 不显示 / 1 记录中 / 2 定时暂停 / 3 一直暂停）该出哪个标记，nil 是不出。
    /// 密码这类私密输入框不出；没开完全访问时键盘不联网、也就谈不上在传，不出。
    static func badge(state: UInt8, privateField: Bool, fullAccess: Bool) -> Badge? {
        guard !privateField, fullAccess else { return nil }
        switch state {
        case 1: return .recording
        case 2, 3: return .paused
        default: return nil
        }
    }
}
