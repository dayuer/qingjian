// MemoryStore 弹给用户的固定文案。写入失败一律以「没存上：」开头再接原因，冲突、读不出、容器不可用各有一句。
// 冲突（conflict）统一说「记忆刚有更新」：记一笔改存素材后，改卡片的只剩键盘新建对象与别的 App 窗口，不再点名是键盘。

extension MemoryStore {
    enum Wording {
        static let noAppGroup = "这个安装包没有开通 App Group，记不了人和事"

        static let unreadable = "记忆暂时读不出来（手机刚开机还没解锁过，或键盘正在写），这时不能改；稍后点「再读一次」"

        static let merged = "记忆刚有更新，已经和你的修改合在一起存好了"

        static let conflictGaveUp = "没存上：记忆刚有更新，已换成最新的内容，请再点一次"

        static let conflictUnreadable = "没存上：记忆刚有更新，重新读取也失败了，请稍后再点一次"

        static let saving = "正在保存"

        static let loading = "正在读取"

        /// 对象设置里「置顶」下面的小字。
        static func pinNote(_ count: Int) -> String {
            count == 0
                ? "键盘上会先摆置顶的人（最多 \(MemoryStore.pinLimit) 个）"
                : "已经置顶 \(count) / \(MemoryStore.pinLimit) 个"
        }

        /// 全局最多置顶几个。
        static func pinLimit() -> String { "最多置顶 \(MemoryStore.pinLimit) 个人" }

        /// 两个开关下面的小字。
        static func switchesNote(_ contact: MemoryContact) -> String {
            "只对\(contact.name)生效。"
        }

        /// 对象设置里「日子提醒」下面的小字，称呼按这个人选的来。
        static func remindOffNote(_ contact: MemoryContact) -> String {
            "关掉后，今天和本周里不再提\(contact.pronoun.label(name: contact.name))的日子"
        }

        static func forgetPartial(_ name: String) -> String {
            "\(name)的卡片已经删了，但名单没更新上，请再点一次「忘掉这个人」"
        }

        static func failed(_ failure: MemoryFailure) -> String { "没存上：\(failure.userMessage)" }

        static func broken(_ names: [String]) -> String {
            "\(names.joined(separator: "、"))的记忆文件坏了，已备份；坏的部分没读进来"
        }
    }
}
