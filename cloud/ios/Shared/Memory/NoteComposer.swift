// 「手写记一笔」的草稿：键区打的字追加到这里而不是宿主。只有一行、光标只在末尾、最多 200 字（MemoryLimits），不支持粘贴。

struct NoteComposer: Equatable {
    private(set) var text = ""

    /// 追加上屏的字。换行去掉（草稿只有一行，换行键在键盘里等于「记到」）；超过 200 字截掉后面的。
    mutating func append(_ input: String) {
        let line = input.filter { !$0.isNewline }
        guard !line.isEmpty else { return }
        text = MemoryLimits.clampText(text + line)
    }

    /// 删末尾一个字（按用户看到的字形算，emoji 带修饰符也是一下删掉）。
    mutating func deleteBackward() {
        guard !text.isEmpty else { return }
        text.removeLast()
    }

    /// 全是空白时不能记（桥也会拒）。
    var canSave: Bool { !text.trimmingCharacters(in: .whitespaces).isEmpty }

    /// 草稿为空时的占位文字。
    static func placeholder(name: String?) -> String {
        let name = name.flatMap { $0.isEmpty ? nil : $0 } ?? "TA"
        return "记一件关于\(name)的事"
    }
}
