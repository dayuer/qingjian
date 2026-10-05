// 草稿卡里的一项（设计稿 01 的 1e-2）：标签、值，以及「拿不准」——拿不准的输入框下面画虚线、
// 右边加个问号、底下写一句为什么拿不准（设计稿的例子：「下个月」换算成「11 月」）。

struct DraftField: Equatable {
    var label: String

    var value: String

    var unsure = false

    /// 拿不准的原因；`unsure` 为真时才显示。
    var why: String?
}
