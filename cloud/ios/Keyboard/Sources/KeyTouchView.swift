// 键区的触摸层：一个 UIKit 视图盖在 SwiftUI 画的键上面，统收全部触摸（仓输入法 / Tasty Imitation Keyboard 的做法）。
// 不用 SwiftUI 手势：它的单指手势在第二根手指落下时会取消第一根（快打丢字母），触摸也要等下一帧才交出来。
// 命中按最近的键（不按行偏移：真机数据里 n 反而常被打成上面的 j，偏移只会更糟）；每根手指各自跟踪，新手指落下时先把上一个键放掉；按下给高亮与反馈，抬起才出字（与系统键盘一致）。
// 背景不能全透明：键盘扩展里透明处的触摸会被系统忽略（Apple 论坛 702798），alpha 0.01 看不见但能收到。

import UIKit

final class KeyTouchView: UIView {
    /// 键区里的格子，坐标以 `keyArea.origin` 为原点；展开面板时为空（面板自己收触摸）。
    var slots: [KeySlot] = []

    /// 键区在本视图里的位置（候选栏下面）。
    var keyArea = CGRect.zero

    /// 候选栏右端 ⌄ 的范围（没在组字时为 nil）：也走这里，SwiftUI 的手势在这个位置常把短点击吞掉。
    var chevron: CGRect?

    var onChevron: (() -> Void)?

    /// 第几格按下（带触摸事件的时间戳，开机时长）/ 抬起 / 被系统取消。
    var onPress: ((Int) -> Void)?

    var onRelease: ((Int, _ cancelled: Bool) -> Void)?

    /// 每根手指按在哪一格。
    private var touched: [ObjectIdentifier: Int] = [:]

    override init(frame: CGRect) {
        super.init(frame: frame)
        isMultipleTouchEnabled = true
        backgroundColor = UIColor.black.withAlphaComponent(0.01)
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    /// 键区与 ⌄ 的触摸归这里，不下钻（下面的 SwiftUI 只画键）；候选栏其余部分、展开的面板仍由 SwiftUI 收。
    override func hitTest(_ point: CGPoint, with event: UIEvent?) -> UIView? {
        if isHidden || !bounds.contains(point) {
            return nil
        }
        if chevron?.contains(point) == true {
            return self
        }
        return !slots.isEmpty && keyArea.contains(point) ? self : nil
    }

    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
        for touch in touches {
            let point = touch.location(in: self)
            if chevron?.contains(point) == true {
                onChevron?()
                continue
            }
            guard let index = nearestSlot(CGPoint(x: point.x - keyArea.minX, y: point.y - keyArea.minY)) else {
                continue
            }
            // 快打时上一个键常常还没抬起：先把它放掉（出字），字母顺序才对
            for (id, held) in touched where held != index {
                touched.removeValue(forKey: id)
                onRelease?(held, false)
            }
            touched[ObjectIdentifier(touch)] = index
            onPress?(index)
        }
    }

    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
        release(touches, cancelled: false)
    }

    override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) {
        release(touches, cancelled: true)
    }

    /// 换层、换面板后格子变了，在押的触摸作废。
    func resetTouches() {
        for index in touched.values {
            onRelease?(index, true)
        }
        touched.removeAll()
    }

    private func release(_ touches: Set<UITouch>, cancelled: Bool) {
        for touch in touches {
            if let index = touched.removeValue(forKey: ObjectIdentifier(touch)) {
                onRelease?(index, cancelled)
            }
        }
    }

    private func nearestSlot(_ point: CGPoint) -> Int? {
        var best: (index: Int, distance: CGFloat)?
        for (index, slot) in slots.enumerated() {
            let dx = max(slot.cell.minX - point.x, 0, point.x - slot.cell.maxX)
            let dy = max(slot.cell.minY - point.y, 0, point.y - slot.cell.maxY)
            let distance = dx * dx + dy * dy
            if best.map({ distance < $0.distance }) ?? true {
                best = (index, distance)
            }
        }
        return best?.index
    }
}
