// 没开完全访问那一屏的颜色选择：这一屏与对象无关，全部用中性色，不引用 accent。

struct NoFullAccessStyle: Equatable {
    /// 候选栏右端的「收起」。
    let closeText: ColorRole

    /// 「开启完全访问后才能用记忆……」那句说明。
    let explanation: ColorRole

    /// 「去开启」描边按钮的文字与描边。
    let buttonText: ColorRole

    let buttonStroke: ColorRole

    /// 点「去开启」后展开的设置路径。
    let path: ColorRole

    static let standard = NoFullAccessStyle(closeText: .ink, explanation: .ink2, buttonText: .ink, buttonStroke: .ink2, path: .ink)

    var roles: [ColorRole] { [closeText, explanation, buttonText, buttonStroke, path] }
}
