// 候选栏下面那块区域现在显示什么。

enum KeyboardPanel: Hashable {
    case keys

    /// 点候选栏右端 ⌄ 展开的全部候选。
    case candidates

    case emoji

    /// 提示行「展开」打开的对象卡。
    case contactCard

    /// 记一笔点「记到 X」后的草稿卡（设计稿 1e-2）。
    case draft

    /// 新内容和已有记忆冲突时的并排选择（设计稿 1e-3）。
    case conflict
}
