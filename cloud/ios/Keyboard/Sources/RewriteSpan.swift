// 改写选哪一段的规则与上屏校验，纯逻辑放这里给单测（KeyboardModel 不进测试 target）。
// 两条规则：宿主里有选中就只改选中；没有就改「当前整句」——从光标往前到最近的一个句末标点
// （。！？!?；;）或换行为止。这句**两头的空白与句末标点不算原文**：开头的缩进与结尾的标点
// 记成 head / tail，发给模型的是干净句子，应用时原样接回去——标点不丢，开头也不被模型吞掉。

import Foundation

struct RewriteSpan: Equatable {
    /// 原文前面紧跟、要原样保留的空白（整句态才有；句首缩进、上一句结尾留下的空格都在这里）。
    let head: String

    /// 要发给模型、要被替换的原文（不含两头的空白与句末标点）。
    let original: String

    /// 原文后面紧跟、要原样保留的句末标点与空白（整句态才有，选中态恒空）。
    let tail: String

    /// 选中态还是整句态。
    let isSelection: Bool

    /// 句末标点；换行也算边界，单列。
    static let terminals: Set<Character> = ["。", "！", "？", "!", "?", ";", "；"]

    init(original: String, head: String = "", tail: String = "", isSelection: Bool) {
        self.head = head
        self.original = original
        self.tail = tail
        self.isSelection = isSelection
    }

    /// 选段。`selection` 是宿主当前选中的文字；选中全是空白（或空串）当没选中，
    /// 落到整句这条路上（那种「选中」多半是误触）。
    /// 整句找不到非空内容时返回 `nil`（不改写）。
    static func select(before: String, selection: String?) -> RewriteSpan? {
        if let selection, !selection.isEmpty, !selection.allSatisfy(\.isWhitespace) {
            return RewriteSpan(original: selection, isSelection: true)
        }
        var body = Substring(before)
        // 尾巴：光标前紧挨着的句末标点与空白——它们属于这句的收尾，不发给模型但要留着
        var tail = ""
        while let last = body.last, terminals.contains(last) || last.isWhitespace {
            tail.insert(last, at: tail.startIndex)
            body = body.dropLast()
        }
        // 边界：剩余部分里最近的一个句末标点或换行；找不到就从这段的开头（聊天框里没有换行的长段，
        // 或光标停在换行后的空白里——那时改的就是上一行的整句，同样讲得通）。
        if let cut = body.lastIndex(where: { terminals.contains($0) || $0 == "\n" }) {
            body = body[body.index(after: cut)...]
        }
        // 脑袋：截断处到正文之间的空白（「上一句。 这一句」中间那个空格）。模型容易把它吃掉，
        // 收起来上屏时接回去
        var head = ""
        while let first = body.first, first.isWhitespace {
            head.append(first)
            body = body.dropFirst()
        }
        guard !body.isEmpty else { return nil }
        return RewriteSpan(original: String(body), head: head, tail: tail, isSelection: false)
    }

    /// 应用前校验：那段字还原样在宿主里才动手。
    /// 选中态看选中没变；整句态看「脑袋 + 原文 + 尾巴」还缀在光标前。
    func applies(before: String, selection: String?) -> Bool {
        if isSelection {
            return selection == original
        }
        return before.hasSuffix(head + original + tail)
    }

    /// 应用时要退格多少个字符（整句态连脑袋与尾巴一起删）。
    var deleteCount: Int { isSelection ? 0 : head.count + original.count + tail.count }

    /// 上屏的最终文字（整句态把两头的空白与标点接回去）。
    func committed(_ result: String) -> String { isSelection ? result : head + result + tail }
}
