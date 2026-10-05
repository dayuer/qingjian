// 开通与加入的状态：桥的调用都阻塞网络，放进后台任务；令牌不经过这里（桥直接写 cloud.toml）。
// 加入是两步：输码拿到 request_id 与 secret，再按 SpaceFlow 的节奏轮询到有结果；对方一直不处理就等到过期。

import Foundation
import Observation
import UIKit

@MainActor
@Observable
final class SpaceStore {
    /// 本机有没有拿到过会话；`nil` 表示还没读。
    private(set) var signedIn: Bool?

    /// 正在等服务器。
    private(set) var busy = false

    /// 输码之后的等待态；`false` 表示在输码那一步。
    private(set) var waiting = false

    /// 给用户看的结果或失败原因。
    var message: String?

    /// 用户是否勾了出境同意。不持久化：每次进来都要重新勾。
    var consented = false

    /// 加入成功、开通成功之后置一次，页面据此 dismiss。
    private(set) var finished = false

    /// 打开的轮询任务；再次输码、取消、页面消失时都要停掉。
    @ObservationIgnored private var polling: Task<Void, Never>?

    /// cloud.toml 的位置；测试里换成临时目录。
    @ObservationIgnored var fileProvider: () -> URL? = { SharedStore.cloudFile }

    /// 这个安装包没有开通 App Group 时的说法，页面上也显示它。
    nonisolated static let noGroup = "这个安装包没有开通 App Group，云服务用不了"

    /// 进这两页时清掉上一次留下的提示：两页共用一个 store，上一步的失败不该跟到下一步。
    func clearMessage() {
        message = nil
    }

    /// 读一次本机的登录状态（只读文件，不联网）。
    func refresh() {
        guard let file = fileProvider() else {
            signedIn = false
            message = Self.noGroup
            return
        }
        signedIn = PairBridge.signedIn(file)
    }

    /// 建空间：成功这台设备就已登录。
    func create() async {
        let consented = consented
        let failure = await run {
            PairBridge.createSpace($0, device: UIDevice.current.name, crossBorderConsented: consented)
        }
        message = failure.map(SpaceWording.failure) ?? SpaceWording.created
        if failure == nil {
            finished = true
            refresh()
        }
    }

    /// 输码申请加入，成功后开始等对方允许。不够 8 位在本地就挡住，不碰桥。
    func join(code: String) async {
        guard MatchCode.isComplete(code) else {
            message = SpaceWording.missingCode
            return
        }
        waiting = true
        message = nil
        let device = UIDevice.current.name
        let entered = MatchCode.entered(code)
        let result = await call { PairBridge.pairJoin($0, code: entered, device: device) }
        switch result {
        case .failure(let failure):
            waiting = false
            message = SpaceWording.failure(failure)
        case .success(let ticket):
            startPolling(ticket)
        }
    }

    /// 不再等：停掉轮询，回到输码那一步。
    func cancelWaiting() {
        polling?.cancel()
        polling = nil
        waiting = false
        message = nil
    }

    private func startPolling(_ ticket: PairReply.Ticket) {
        polling?.cancel()
        polling = Task { [weak self] in
            let flow = SpaceFlow(expiresAt: ticket.expiresAt)
            while !Task.isCancelled, flow.shouldPoll(now: Self.now) {
                try? await Task.sleep(for: .seconds(flow.nextDelay(now: Self.now)))
                if Task.isCancelled { return }
                switch await self?.pollOnce(ticket) {
                case .approved:
                    await self?.finishJoining(SpaceWording.joined)
                    return
                case .cancelled, .none:
                    return
                case .denied:
                    await self?.finishJoining(SpaceWording.denied)
                    return
                case .pending:
                    continue
                }
            }
            await self?.giveUpWaiting()
        }
    }

    private enum PollOutcome: Equatable {
        case pending
        case denied
        case approved
        case cancelled
    }

    private func pollOnce(_ ticket: PairReply.Ticket) async -> PollOutcome {
        guard let file = fileProvider() else { return .cancelled }
        let result = await Task.detached(priority: .userInitiated) {
            PairBridge.pairPoll(file, requestId: ticket.requestId, secret: ticket.secret)
        }.value
        switch result {
        case .failure:
            // 单次失败不打断等待：网络抖一下不该让用户重输码。放弃是走到过期那一步的事。
            return .pending
        case .success(let poll):
            switch poll.state {
            case .pending: return .pending
            case .denied: return .denied
            case .approved: return .approved
            }
        }
    }

    private func finishJoining(_ text: String) {
        polling = nil
        waiting = false
        message = text
        if text == SpaceWording.joined {
            finished = true
            refresh()
        }
    }

    private func giveUpWaiting() {
        polling = nil
        waiting = false
        message = SpaceWording.expired
    }

    /// 当前时间，Unix 毫秒。
    nonisolated static var now: Int64 { Int64(Date().timeIntervalSince1970 * 1000) }

    /// 在后台跑一次桥的操作；成功返回 nil，失败返回原因。
    private func run(_ work: @escaping @Sendable (URL) -> BridgeFailure?) async -> BridgeFailure? {
        guard let file = fileProvider() else {
            message = Self.noGroup
            return BridgeFailure(code: .other, message: Self.noGroup)
        }
        busy = true
        defer { busy = false }
        return await Task.detached(priority: .userInitiated) { work(file) }.value
    }

    /// 同 `run`，但要一个成功值。
    private func call<T: Sendable>(_ work: @escaping @Sendable (URL) -> Result<T, BridgeFailure>) async -> Result<T, BridgeFailure> {
        guard let file = fileProvider() else {
            return .failure(BridgeFailure(code: .other, message: Self.noGroup))
        }
        busy = true
        defer { busy = false }
        return await Task.detached(priority: .userInitiated) { work(file) }.value
    }
}
