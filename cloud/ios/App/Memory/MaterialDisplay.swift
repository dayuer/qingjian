// 「待整理」一节的文字，提成纯函数方便单测：标题条数、每条的时间与来源、收起时只显示前 2 行。
// 时间按北京时间（与记忆里别的日期一致，MemoryDate）；原话本身不改一个字，只决定收起时显示到哪。

import Foundation

enum MaterialDisplay {
    /// 收起时最多显示几行原话。
    static let previewLines = 2

    /// 一行大概放得下的字数（15pt 正文、详情页的行宽）；单行超过两倍也算长，给「展开全文」。
    static let charsPerLine = 22

    static func title(count: Int) -> String { "待整理 · \(count) 条" }

    /// 每条上面的小字：「今天 09:34 · 剪贴板」。
    static func meta(at: Int64, source: MemoryMaterial.Source, now: Date = Date()) -> String {
        "\(time(at: at, now: now)) · \(sourceLabel(source))"
    }

    static func sourceLabel(_ source: MemoryMaterial.Source) -> String {
        switch source {
        case .clipboard: "剪贴板"
        case .typed: "手写"
        }
    }

    /// 今天、昨天写「今天 / 昨天 HH:mm」，今年写「M月d日 HH:mm」，更早的带年份。
    static func time(at: Int64, now: Date = Date()) -> String {
        let date = Date(timeIntervalSince1970: TimeInterval(at))
        let calendar = MemoryDate.calendar
        let parts = calendar.dateComponents([.year, .month, .day, .hour, .minute], from: date)
        let clock = String(format: "%02d:%02d", parts.hour ?? 0, parts.minute ?? 0)
        switch MemoryDate.daysBetween(date, now) {
        case 0: return "今天 \(clock)"
        case 1: return "昨天 \(clock)"
        default:
            let day = "\(parts.month ?? 0)月\(parts.day ?? 0)日 \(clock)"
            return parts.year == calendar.component(.year, from: now) ? day : "\(parts.year ?? 0)年\(day)"
        }
    }

    /// 收起时显示的部分：原话的前 2 行（按换行算），去掉末尾空白；没超过就是原话本身。
    static func preview(_ text: String) -> String {
        let lines = text.components(separatedBy: "\n")
        guard lines.count > previewLines else { return text }
        return lines.prefix(previewLines).joined(separator: "\n").trimmingCharacters(in: .whitespacesAndNewlines)
    }

    /// 收起时看不全：超过 2 行，或者行很长（折行后超过 2 行）。
    static func isLong(_ text: String) -> Bool {
        let wrapped = text.components(separatedBy: "\n").reduce(0) { $0 + max(1, ($1.count + charsPerLine - 1) / charsPerLine) }
        return wrapped > previewLines
    }

    static let expandHint = "展开全文"

    static let collapseHint = "收起"

    static let deleteButton = "删除"

    static let deleteTitle = "删掉这条原话？"

    static let deleteMessage = "删掉后找不回来，也不会再整理成记忆卡。"

    /// 没开素笺云时这一节下面的一行，点了进 CloudIntroView。
    static let cloudHint = "开通素笺云后，每天帮你整理成记忆卡"
}
