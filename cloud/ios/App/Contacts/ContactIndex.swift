// 通讯录（设计稿 02 的 2b）的列表结构：按首字母分组、组内排序、右侧索引的字母、搜索、行尾事件提示。
// 首字母由键盘那一侧用词库算好写在 `MemoryContact.initial` 上（D4：App 不自己算拼音），没有的归「#」。

import Foundation

/// 一个字母小节。
struct ContactSection: Identifiable, Equatable {
    /// 单个大写字母，或「#」。
    let letter: String

    let people: [MemoryContact]

    var id: String { letter }
}

enum ContactIndex {
    /// 首字母算不出来（旧数据、词库不认得的名字）的那一节，排最后。
    static let other = "#"

    /// 分组：组按字母序（「#」最后），组内按名字排。
    static func sections(_ contacts: [MemoryContact]) -> [ContactSection] {
        var buckets: [String: [MemoryContact]] = [:]
        for contact in contacts {
            buckets[letter(of: contact), default: []].append(contact)
        }
        return buckets.keys.sorted(by: isBefore).map { letter in
            ContactSection(letter: letter, people: (buckets[letter] ?? []).sorted(by: byName))
        }
    }

    /// 右侧索引的字母：就是各小节的字母，一个以上才值得显示。
    static func letters(_ sections: [ContactSection]) -> [String] {
        sections.map(\.letter)
    }

    /// 搜索：名字、代号或场景名里含这段字（不分大小写）；空关键词原样返回。
    /// 场景名是用户自己起的，由调用方查（`sceneName`）。
    static func search(
        _ contacts: [MemoryContact], text: String, sceneName: (String) -> String
    ) -> [MemoryContact] {
        let needle = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !needle.isEmpty else { return contacts }
        return contacts.filter { contact in
            [contact.name, contact.displayName ?? "", sceneName(contact.scene)]
                .contains { $0.range(of: needle, options: [.caseInsensitive, .diacriticInsensitive]) != nil }
        }
    }

    /// 搜索框的占位（设计稿 2b：「搜索 7 个人」）。
    static func searchPrompt(count: Int) -> String {
        "搜索 \(count) 个人"
    }

    /// 行尾的事件提示：每个人最近的那一件；`upcoming` 已按天数排好，取每人第一条即可。
    static func notes(_ upcoming: [MemoryUpcoming]) -> [String: String] {
        var notes: [String: String] = [:]
        for item in upcoming where notes[item.contact.id] == nil {
            notes[item.contact.id] = item.rowNote
        }
        return notes
    }

    /// 展开区先给哪几张：最近改过的排前面（「先快速记，再整理」，最近的更可能是刚记下的）。
    static func preview(_ cards: [MemoryCard], limit: Int = 3) -> [MemoryCard] {
        cards.sorted { ($0.touchedAt, $0.id) > ($1.touchedAt, $1.id) }.prefix(limit).map { $0 }
    }

    /// 这个人的分组字母：桥给的是单个大写字母，别的一律归「#」。
    static func letter(of contact: MemoryContact) -> String {
        guard let first = contact.initial?.uppercased().first, first.isLetter, first.isASCII else {
            return other
        }
        return String(first)
    }

    /// 字母序，「#」最后。
    private static func isBefore(_ a: String, _ b: String) -> Bool {
        if a == other { return false }
        if b == other { return true }
        return a < b
    }

    /// 同一个字母里按名字排。名字是汉字时这里按码位，不是拼音——拼音首字母已经分好组了，组内先后不影响找谁。
    private static func byName(_ a: MemoryContact, _ b: MemoryContact) -> Bool {
        let order = a.name.localizedStandardCompare(b.name)
        return order == .orderedSame ? a.id < b.id : order == .orderedAscending
    }
}
