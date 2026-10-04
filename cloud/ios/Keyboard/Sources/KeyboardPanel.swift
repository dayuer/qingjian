// 候选栏下面那块区域现在显示什么。

enum KeyboardPanel: Hashable {
    case keys

    /// 点候选栏右端 ⌄ 展开的全部候选。
    case candidates

    case emoji

    /// 点场景牌子打开的场景 / 对象选择。
    case scope

    /// 提示行「展开」打开的对象卡。
    case contactCard
}
