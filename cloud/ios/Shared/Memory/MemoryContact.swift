// 记忆里的一个人（contacts.json 的一项）。名字只在这里，对象目录名用随机 id。两个提示开关按人设置，旧文件没有时按开。
// `pinnedAt` 是他被置顶的时间（全局最多 4 个）。
// 键盘上画出来的称呼一律用 `chipName`（代号优先），App 里照旧显示 `name`；Tests/KeyboardNameGuardTests 守着键盘源码不直接读 `.name`。

import Foundation

struct MemoryContact: Codable, Identifiable, Hashable, Sendable {
    let id: String

    var name: String

    /// 键盘上显示的代号（桥的 display_name，最多 12 字）；nil 时键盘显示名字。
    var displayName: String?

    /// 名字首字的首字母（桥的 `initial`）：通讯录按它分组、做右侧索引（设计稿 02 的 2b）。
    /// 由键盘那侧用词库算好写进 contacts.json（D4：App 不自己算拼音）；旧数据与词库不认得的名字没有，按「#」归。
    var initial: String?

    var pronoun: MemoryPronoun

    /// 置顶的时间（Unix 秒）：键盘上先摆置顶的人，全局最多 4 个；nil 就是没置顶。
    var pinnedAt: Int64?

    let createdAt: Int64

    /// 打字时按这个人的卡片提示。
    var hintOn = true

    /// 这个人的日子与约定快到时提醒。
    var remindOn = true

    enum CodingKeys: String, CodingKey {
        case id, name, initial, pronoun
        case displayName = "display_name"
        case pinnedAt = "pinned_at"
        case createdAt = "created_at"
        case hintOn = "hint_on"
        case remindOn = "remind_on"
    }

    /// 新对象：id 用随机 32 位十六进制（对象目录名就是它）。
    static func new(name: String, pronoun: MemoryPronoun) -> MemoryContact {
        MemoryContact(
            id: MemoryID.make(), name: name, pronoun: pronoun,
            createdAt: Int64(Date().timeIntervalSince1970))
    }

    /// 键盘上画出来、读出来、写进日志的称呼：代号去掉首尾空白后不是空的就用代号，否则用名字（与桥的 `chip_name` 一致）。
    var chipName: String { Self.chipName(displayName: displayName, name: name) }

    static func chipName(displayName: String?, name: String) -> String {
        let code = displayName?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        return code.isEmpty ? name : code
    }

    /// 认识了几天：按北京时间的日历日，建的那天算第 1 天。
    func knownDays(now: Date = Date()) -> Int {
        MemoryDate.daysBetween(Date(timeIntervalSince1970: TimeInterval(createdAt)), now) + 1
    }
}

extension MemoryContact {
    /// 缺 `hint_on` / `remind_on` 的旧文件按开（init 写在扩展里，成员逐一构造器才留得住）。
    init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        self.init(
            id: try container.decode(String.self, forKey: .id),
            name: try container.decode(String.self, forKey: .name),
            displayName: try container.decodeIfPresent(String.self, forKey: .displayName),
            initial: try container.decodeIfPresent(String.self, forKey: .initial),
            pronoun: try container.decodeIfPresent(MemoryPronoun.self, forKey: .pronoun) ?? .ta,
            pinnedAt: try container.decodeIfPresent(Int64.self, forKey: .pinnedAt),
            createdAt: try container.decode(Int64.self, forKey: .createdAt),
            hintOn: try container.decodeIfPresent(Bool.self, forKey: .hintOn) ?? true,
            remindOn: try container.decodeIfPresent(Bool.self, forKey: .remindOn) ?? true)
    }
}
