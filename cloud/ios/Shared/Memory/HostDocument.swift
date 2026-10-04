// 宿主输入框有没有换：textDocumentProxy.documentIdentifier 每个输入框一个，我们自己上屏不会改它。

import Foundation

enum HostDocument {
    /// 变了（包括第一次看到、离开输入框）就清：没上屏的拼音丢掉，桥里最近上屏的字也清掉，
    /// 免得旧拼音上屏到新输入框、在 A 聊天里打的字在 B 里触发记忆提示。真机上同一个 App 里换输入框标识也会变。
    static func changed(from last: UUID?, to current: UUID?) -> Bool { last != current }
}
