// MaterialsStore 读删素材用的两个函数：正式走桥（MaterialFiles），测试换成假的，好把读不出、删失败这些路径测到。
// 在 MaterialsWorker 的后台队列上调用，所以要 Sendable。

import Foundation

struct MaterialsBackend: Sendable {
    /// 读一个人没整理的素材；读不出为 nil。
    var list: @Sendable (_ directory: URL, _ contactId: String) -> MaterialList?

    /// 删一条；成功为 nil。
    var delete: @Sendable (_ directory: URL, _ contactId: String, _ clientId: String) -> MemoryFailure?

    static var bridge: MaterialsBackend {
        MaterialsBackend(
            list: { MaterialFiles.list(userDirectory: $0, contactId: $1) },
            delete: { MaterialFiles.delete(userDirectory: $0, contactId: $1, clientId: $2) })
    }
}
