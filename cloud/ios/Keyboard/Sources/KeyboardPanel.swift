// 候选栏下面那块区域现在显示什么。

enum KeyboardPanel: Hashable {
    case keys

    /// 点候选栏右端 ⌄ 展开的全部候选。
    case candidates

    case emoji
}
