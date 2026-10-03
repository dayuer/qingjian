// 键盘往宿主应用输出文字的出口；控制器用 textDocumentProxy 实现。

@MainActor
protocol TextOutput: AnyObject {
    /// 把组字中的拼音以 marked text 显示在宿主的光标处；空串表示清掉。
    func setMarked(_ text: String)

    /// 上屏：有 marked text 时替换它，没有时在光标处插入。
    func commit(_ text: String)

    func deleteBackward()

    func switchToNextKeyboard()
}
