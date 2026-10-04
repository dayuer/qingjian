// 改写（润色）在界面上的状态：没在组字时工具栏显示「改写」，点了等结果，结果出来点一下替换原文。

enum RewriteState: Equatable {
    case idle

    /// 已发出，`original` 是要被替换的那段（光标前的文字）。
    case pending(original: String)

    case ready(original: String, result: String)

    case failed
}
