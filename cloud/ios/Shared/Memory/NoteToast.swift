// 记一笔条上的一行短提示：记下了（圆点用灰绿，2 秒），或者没记上的原因（灰色圆点，停久一点好看清）。

import Foundation

struct NoteToast: Equatable {
    let text: String

    /// 没记上（满了、补写被拒绝）：圆点用灰色，不用代表人的灰绿。
    let warning: Bool

    /// 在条上停多久。
    var duration: Duration { warning ? .seconds(4) : .seconds(2) }

    static func done(count: Int, cloud: Bool) -> NoteToast {
        NoteToast(text: NoteBarText.doneText(count: count, cloud: cloud), warning: false)
    }

    static func info(_ text: String) -> NoteToast {
        NoteToast(text: text, warning: false)
    }

    static func problem(_ text: String) -> NoteToast {
        NoteToast(text: text, warning: true)
    }
}
