// 宿主输入框有没有换：textDocumentProxy.documentIdentifier 每个输入框一个，我们自己上屏不会改它。

import Foundation

enum HostDocument {
    /// 变了（包括第一次看到）就该让桥清掉最近上屏的字，免得在 A 聊天里打的字在 B 里触发记忆提示。
    static func changed(from last: UUID?, to current: UUID?) -> Bool { last != current }
}
