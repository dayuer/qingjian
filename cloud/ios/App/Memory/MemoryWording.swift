// MemoryStore 弹给用户的固定文案。写入失败一律以「没存上：」开头再接原因，冲突、读不出、容器不可用各有一句。

extension MemoryStore {
    enum Text {
        static let noAppGroup = "这个安装包没有开通 App Group，记不了人和事"

        static let unreadable = "记忆暂时读不出来（手机刚开机还没解锁过，或键盘正在写），这时不能改；稍后点「再读一次」"

        static let merged = "键盘刚记过一笔，已经和你的修改合在一起存好了"

        static let conflictGaveUp = "没存上：键盘一直在改记忆，已换成最新的内容，请再改一次"

        static let conflictUnreadable = "没存上：键盘刚改过记忆，重新读取也失败了，请稍后再试"

        static func failed(_ failure: MemoryFailure) -> String { "没存上：\(failure.userMessage)" }

        static func broken(_ names: [String]) -> String {
            "\(names.joined(separator: "、"))的记忆文件坏了，已备份；坏的部分没读进来"
        }
    }
}
