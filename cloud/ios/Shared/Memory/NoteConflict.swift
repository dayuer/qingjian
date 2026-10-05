// 记一笔和已有记忆冲突时的那一对（设计稿 01 的 1e-3）：旧的那张、新的那句，以及新的落在哪个对象上。
// 新内容的时间标签由调用方写好（设计稿写「新的 · 今天」）。

struct NoteConflict: Equatable {
    /// 冲突发生在谁身上（写进「和小美的一条记忆说法不一样」）；键盘上的称呼，用 `MemoryContact.chipName`。
    var contactName: String

    /// 旧卡：时间标签与原文（原文带删除线）。
    var oldLabel: String
    var oldText: String

    /// 新的：时间标签与文字。
    var newLabel: String
    var newText: String
}
