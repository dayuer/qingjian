// 对象详情「待整理」一节的数据：一个人没整理的素材（原话），经 MaterialsWorker 在后台读删。
// 读不出来常驻在这一节里（带「再读一次」），删失败弹一句「没删掉：原因」，都不静默。键盘随时可能再记一笔，App 回到前台时重读。

import Foundation
import Observation

@MainActor
@Observable
final class MaterialsStore {
    /// 读到的素材；还没读过为 nil。
    private(set) var list: MaterialList?

    /// 读不出来的原因；有它时这一节只显示原因与「再读一次」。
    private(set) var loadError: String?

    /// 正在删的那条，删完前它的删除按钮置灰。
    private(set) var deleting: String?

    /// 要弹给用户的话（删失败）。
    var message: String?

    @ObservationIgnored private let directory: () -> URL?

    @ObservationIgnored private let worker: MaterialsWorker

    @ObservationIgnored private var reading = false

    init(directory: @escaping () -> URL? = { SharedStore.directory }, backend: MaterialsBackend = .bridge) {
        self.directory = directory
        self.worker = MaterialsWorker(backend: backend)
    }

    /// 没整理的条数；还没读过时为 0。
    var count: Int { list?.unprocessedCount ?? 0 }

    func reload(_ contactId: String) async {
        guard let directory = directory() else {
            loadError = MemoryStore.Wording.noAppGroup
            return
        }
        guard !reading else { return }
        reading = true
        let next = await worker.list(directory, contactId: contactId)
        reading = false
        guard let next else {
            loadError = Wording.unreadable
            return
        }
        list = next
        loadError = nil
    }

    /// 删一条，成功后重读（条数跟着变）；失败弹原因，列表不动。
    @discardableResult
    func delete(_ material: MemoryMaterial, contactId: String) async -> Bool {
        guard deleting == nil else { return false }
        guard let directory = directory() else {
            message = MemoryStore.Wording.noAppGroup
            return false
        }
        deleting = material.id
        let failure = await worker.delete(directory, contactId: contactId, clientId: material.clientId)
        deleting = nil
        if let failure {
            message = Wording.deleteFailed(failure)
            return false
        }
        await reload(contactId)
        return true
    }

    /// 测试用：不经桥直接换数据。
    func replace(with list: MaterialList) {
        self.list = list
        loadError = nil
    }
}
