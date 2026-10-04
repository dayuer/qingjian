// 一张记忆卡（<对象 id>/cards.json 的一项），与桥的 Card 一一对应。文字最多 200 字、关键词最多 8 个（见 MemoryLimits）。
// faded / seq / updatedAt 是 2C 云端卡的字段，手写卡恒为 false / 0 / 0，读进来原样写回，不丢。

import Foundation

struct MemoryCard: Codable, Identifiable, Hashable, Sendable {
    let id: String

    var kind: Kind

    var text: String

    var keywords: [String]

    /// `yyyy-MM-dd`（北京时间），只对日子与约定有意义。
    var when: String?

    var source: String

    var confirmed: Bool

    /// 已淡出（过期不再提示）。
    var faded = false

    /// 服务端序号，每个用户单独递增。
    var seq: Int64 = 0

    /// 服务端记的最后修改时间，Unix 毫秒。
    var updatedAt: Int64 = 0

    /// 建卡时间，Unix 秒。
    let createdAt: Int64

    /// 本地最后一次改的时间，Unix 秒；写回冲突时两边都改了以它新者为准。
    var touchedAt: Int64

    enum CodingKeys: String, CodingKey {
        case id, kind, text, keywords, when, source, confirmed, faded, seq
        case updatedAt = "updated_at"
        case createdAt = "created_at"
        case touchedAt = "touched_at"
    }

    enum Kind: String, Codable, CaseIterable, Sendable {
        case date, promise, preference, recent, other

        var title: String {
            switch self {
            case .date: "日子"
            case .promise: "约定"
            case .preference: "喜好"
            case .recent: "近况"
            case .other: "其他"
            }
        }

        var hasDate: Bool { self == .date || self == .promise }
    }

    /// 手写的新卡。
    static func new(kind: Kind, text: String, when: String?, keywords: [String]) -> MemoryCard {
        let now = Int64(Date().timeIntervalSince1970)
        return MemoryCard(
            id: MemoryID.make(), kind: kind, text: text, keywords: keywords, when: when,
            source: "manual", confirmed: true, createdAt: now, touchedAt: now)
    }

    /// 离今天还有几天：日子按年重复（今年那天过了看明年），约定按写的那天；别的种类、日期写错为 nil。与桥的 `days_away` 一致。
    func daysAway(now: Date = Date()) -> Int? {
        guard let when, let date = MemoryDate.parse(when) else { return nil }
        switch kind {
        case .date: return MemoryDate.daysBetween(now, MemoryDate.nextAnniversary(of: date, from: now))
        case .promise: return MemoryDate.daysBetween(now, date)
        default: return nil
        }
    }

    /// 日子「明天是她的生日」，约定「明天：看电影」：与桥的 `reminder_text` 同一模板（扩展之前的最后一个方法）。
    func reminderText(days: Int, contact: MemoryContact) -> String {
        let when = switch days {
        case 0: "今天"
        case 1: "明天"
        default: "\(days) 天后"
        }
        if kind == .promise { return "\(when)：\(text)" }
        return "\(when)是\(contact.pronoun.label(name: contact.name))的\(text)"
    }
}

extension MemoryCard {
    /// 缺字段的旧文件按缺省（init 写在扩展里，成员逐一构造器才留得住）。
    init(from decoder: any Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        self.init(
            id: try container.decode(String.self, forKey: .id),
            kind: try container.decode(Kind.self, forKey: .kind),
            text: try container.decode(String.self, forKey: .text),
            keywords: try container.decodeIfPresent([String].self, forKey: .keywords) ?? [],
            when: try container.decodeIfPresent(String.self, forKey: .when),
            source: try container.decodeIfPresent(String.self, forKey: .source) ?? "manual",
            confirmed: try container.decodeIfPresent(Bool.self, forKey: .confirmed) ?? false,
            faded: try container.decodeIfPresent(Bool.self, forKey: .faded) ?? false,
            seq: try container.decodeIfPresent(Int64.self, forKey: .seq) ?? 0,
            updatedAt: try container.decodeIfPresent(Int64.self, forKey: .updatedAt) ?? 0,
            createdAt: try container.decode(Int64.self, forKey: .createdAt),
            touchedAt: try container.decode(Int64.self, forKey: .touchedAt))
    }
}
