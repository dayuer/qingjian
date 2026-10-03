// 键盘与宿主应用、系统剪贴板之间的出入口；控制器用 textDocumentProxy 与 UIPasteboard 实现。

@MainActor
protocol TextOutput: AnyObject {
    /// 把组字中的拼音以 marked text 显示在宿主的光标处；空串表示清掉。
    func setMarked(_ text: String)

    /// 上屏：有 marked text 时替换它，没有时在光标处插入。
    func commit(_ text: String)

    func deleteBackward()

    /// 宿主光标前的文字（可能含我们写的 marked text），拿不到时是空串。
    var contextBefore: String { get }

    var contextAfter: String { get }

    func switchToNextKeyboard()

    /// 系统剪贴板的变化计数；只读计数不会弹系统的粘贴授权提示。
    var pasteboardChangeCount: Int { get }

    var pasteboardHasText: Bool { get }

    /// 读剪贴板里的文字（可能弹系统的粘贴授权提示，只在用户点了按钮后调）。
    func readPasteboard() -> String?
}
