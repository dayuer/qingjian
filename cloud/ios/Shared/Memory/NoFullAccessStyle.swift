// 没开完全访问那一屏的颜色选择：这一屏与对象无关，全部用中性色，不引用 accent。

struct NoFullAccessStyle: Equatable {
    /// 候选栏右端的「收起」。
    let closeText: ColorRole

    /// 「去开启」描边按钮的文字与描边。
    let buttonText: ColorRole

    let buttonStroke: ColorRole

    /// 点按钮后的一行提示。
    let notice: ColorRole

    static let standard = NoFullAccessStyle(closeText: .ink, buttonText: .ink, buttonStroke: .ink2, notice: .ink2)

    var roles: [ColorRole] { [closeText, buttonText, buttonStroke, notice] }
}
