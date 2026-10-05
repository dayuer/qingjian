// 功勋路（设计稿 02 的 2a）：跟一个人走过的节点，两种，都照 2026-10-05 设计稿答的 D3 ——
//   一是认识天数：满 100、200 天与一周年各亮一次；
//   二是确认过的卡片数：满 10、50、100、200 张各亮一次。
// **不做连续打卡**（断了不扣，不给人负担）。原点「认识」那天本身也是一个节点。
//
// 画法：已亮的黑点 + 名字 + 达成那天的日期；**最近的下一站**用灰绿点加一圈浅灰绿光晕、名字用灰绿字，
// 下面不写日期、写还差多少（「还差 4 张」「还差 6 天」）。已亮的按时间排，下一站排在最后。
//
// 纯值，不碰 UI 与存储，好单测（见 Tests/MilestonesTests）。

import Foundation

/// 功勋路上的一个节点。
struct Milestone: Identifiable, Equatable {
    /// 哪种节点。`met` 是原点（认识那天）。
    enum Kind: Equatable {
        case met
        case days(Int)
        case cards(Int)
    }

    let kind: Kind

    /// 达成那天；未达成的按 `Date.distantFuture` 排到最后。
    let date: Date

    /// 节点名：「认识」「认识 100 天」「记满 10 张」。
    let title: String

    /// 已经亮了没有。
    let earned: Bool

    /// 未达成时下面那行；已达成为 nil（那时下面写日期）。
    let remaining: String?

    var id: String { title }
}

enum Milestones {
    /// 认识的节点（天数，含原点）。
    static let daySteps = [100, 200, 365]

    /// 记满多少张卡算一个节点。
    static let cardSteps = [10, 50, 100, 200]

    /// 这个人的功勋路：已亮的按时间排 + 最近的下一站（没有下一站就只到最后一个）。
    static func road(contact: MemoryContact, cards: [MemoryCard], now: Date = Date()) -> [Milestone] {
        let calendar = MemoryDate.calendar
        let created = Date(timeIntervalSince1970: TimeInterval(contact.createdAt))
        let known = contact.knownDays(now: now)
        // 只算确认过的卡（云端整理出来还没确认的不算数）
        let confirmed = cards.filter(\.confirmed).sorted { $0.createdAt < $1.createdAt }

        var earned: [Milestone] = [
            Milestone(kind: .met, date: created, title: "认识", earned: true, remaining: nil)
        ]
        var pending: [Milestone] = []

        for days in daySteps {
            // 认识那天算第 1 天，所以满 n 天是建的那天往后 n-1 天
            let date = calendar.date(byAdding: .day, value: days - 1, to: created) ?? created
            let reached = known >= days
            let milestone = Milestone(
                kind: .days(days), date: reached ? date : .distantFuture,
                title: "认识 \(days) 天", earned: reached,
                remaining: reached ? nil : "还差 \(days - known) 天")
            reached ? earned.append(milestone) : pending.append(milestone)
        }

        for count in cardSteps {
            let reached = confirmed.count >= count
            // 达成那天就是第 n 张卡建的那天
            let date = reached
                ? Date(timeIntervalSince1970: TimeInterval(confirmed[count - 1].createdAt))
                : Date.distantFuture
            let milestone = Milestone(
                kind: .cards(count), date: date, title: "记满 \(count) 张", earned: reached,
                remaining: reached ? nil : "还差 \(count - confirmed.count) 张")
            reached ? earned.append(milestone) : pending.append(milestone)
        }

        let sorted = earned.sorted { $0.date < $1.date }
        guard let next = nearest(pending, known: known, confirmed: confirmed.count) else {
            return sorted
        }
        return sorted + [next]
    }

    /// 「最近的下一站」取**完成度最高**的那个（记满 50 张 差 4 张 比 认识 365 天 差 151 天近）。
    ///
    /// 这是从设计稿那张图反推的：稿里认识 214 天、46 张卡，下一站画的是「记满 50 张」（46/50 = 0.92，
    /// 而 214/365 = 0.59）。设计说明只写「浅绿点是最近的下一站」，没给算法，**这条待确认**。
    private static func nearest(
        _ pending: [Milestone], known: Int, confirmed: Int
    ) -> Milestone? {
        func progress(_ milestone: Milestone) -> Double {
            switch milestone.kind {
            case .met: return 1
            case .days(let days): return Double(known) / Double(days)
            case .cards(let count): return Double(confirmed) / Double(count)
            }
        }
        func threshold(_ milestone: Milestone) -> Int {
            switch milestone.kind {
            case .met: return 0
            case .days(let days): return days
            case .cards(let count): return count
            }
        }
        // 同进度时取门槛小的，结果稳定
        return pending.sorted {
            let left = progress($0)
            let right = progress($1)
            return left == right ? threshold($0) < threshold($1) : left > right
        }.first
    }
}
