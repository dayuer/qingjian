// 账号页的状态：登录状态、登录方式、设备与四个开关都从桥取（桥带着 App Group 里 cloud.toml 的令牌去问服务器）。
// 桥的调用都阻塞网络，放进后台任务；改动成功后整页重取一遍。令牌不经过这里。

import AuthenticationServices
import Foundation
import Observation
import UIKit

@MainActor
@Observable
final class AccountStore {
    private(set) var state: AccountState?

    /// 正在等服务器：页面禁用操作。
    private(set) var busy = false

    /// 给用户看的结果或失败原因，显示在页面顶上。
    var message: String?

    /// 邮箱登录走到哪一步，以及两个输入框的内容（放在这里，失败时才能清空验证码）。
    var loginStep = LoginStep.email

    var email = ""

    var code = ""

    /// 当天验证失败太多次被锁：邮箱区域禁用，只能用 Apple 登录。
    private(set) var emailLocked = false

    /// 每加一次，验证码输入框抢一次焦点。
    private(set) var codeFocusTick = 0

    /// 这一次 Apple 登录的原始 nonce；请求里给 Apple 的是它的 SHA-256。
    private var appleNonce: String?

    private static let noGroup = "这个安装包没有开通 App Group，账号用不了"

    func refresh() async {
        guard let file = SharedStore.cloudFile else {
            message = Self.noGroup
            return
        }
        SharedStore.ensure(SharedStore.directory)
        let next = await Task.detached(priority: .userInitiated) { AccountBridge.status(file) }.value
        guard let next else {
            message = "账号信息读不出来"
            return
        }
        state = next
        if next.signedIn { emailLocked = false }
        if let error = next.error { message = error }
    }

    /// 失败的 code 加登录步骤 → 界面动作；message 总是显示。
    nonisolated static func reaction(for failure: AccountFailure, step: LoginStep) -> AccountReaction {
        switch failure.code {
        case .authFailed: step == .code ? .retryCode : .showMessage
        case .lockedToday: .lockEmail
        case .unauthorized: .signOutLocally
        case .notConfigured, .rateLimited, .forbidden, .unreachable, .invalidArgument, .notSignedIn, .other:
            .showMessage
        }
    }

    /// SignInWithAppleButton 发请求前调：生成这一次的 nonce。
    func prepare(_ request: ASAuthorizationAppleIDRequest) {
        let nonce = Nonce.random()
        appleNonce = nonce
        request.requestedScopes = [.email]
        request.nonce = Nonce.sha256Hex(nonce)
    }

    func completeApple(_ result: Result<ASAuthorization, any Error>) async {
        let authorization: ASAuthorization
        switch result {
        case .success(let value):
            authorization = value
        case .failure(let error):
            if (error as? ASAuthorizationError)?.code != .canceled {
                message = "Apple 登录没有完成：\(error.localizedDescription)"
            }
            return
        }
        guard let credential = authorization.credential as? ASAuthorizationAppleIDCredential,
              let token = credential.identityToken.flatMap({ String(data: $0, encoding: .utf8) }),
              let code = credential.authorizationCode.flatMap({ String(data: $0, encoding: .utf8) }),
              let nonce = appleNonce
        else {
            message = "Apple 没有给出登录凭据，请重试"
            return
        }
        appleNonce = nil
        let device = UIDevice.current.name
        if await perform({
            AccountBridge.signInApple(
                $0, identityToken: token, authorizationCode: code, nonce: nonce, device: device)
        }) {
            message = "已登录"
        }
    }

    /// 发验证码；成功返回 true。
    func emailStart(_ email: String) async -> Bool {
        let sent = await perform({ AccountBridge.emailStart($0, email: email) }, refreshAfter: false)
        if sent {
            loginStep = .code
            message = "验证码已发出，10 分钟内有效"
        }
        return sent
    }

    /// 用验证码登录；成功返回 true。
    func emailVerify(_ email: String, code: String) async -> Bool {
        let device = UIDevice.current.name
        let signedIn = await perform({
            AccountBridge.emailVerify($0, email: email, code: code, device: device)
        })
        if signedIn {
            loginStep = .email
            self.code = ""
            message = "已登录"
        }
        return signedIn
    }

    /// 先改界面再问服务器，失败就改回去。
    func setConsent(_ feature: CloudFeature, _ enabled: Bool) async {
        guard let previous = state?.consents else { return }
        state?.consents[feature] = enabled
        let name = feature.rawValue
        let changed = await perform({ AccountBridge.setConsent($0, feature: name, enabled: enabled) })
        // 令牌失效时 perform 已把页面刷成未登录，不要把旧开关写回去
        if !changed, state?.signedIn == true { state?.consents = previous }
    }

    func revoke(_ device: AccountDevice) async {
        let id = device.id
        if await perform({ AccountBridge.revokeSession($0, id: id) }) {
            message = "已注销「\(device.name)」"
        }
    }

    func signOut() async {
        if await perform({ AccountBridge.signOut($0) }) { message = "已退出登录" }
    }

    func deleteAccount() async {
        if await perform({ AccountBridge.deleteAccount($0) }) {
            message = "账号已删除，服务器上的数据已清除"
        }
    }

    /// 在后台跑一次桥的操作；失败时按 code 处理并显示 message。成功后默认整页重取。
    @discardableResult
    private func perform(
        _ work: @escaping @Sendable (URL) -> AccountFailure?, refreshAfter: Bool = true
    ) async -> Bool {
        guard let file = SharedStore.cloudFile else {
            message = Self.noGroup
            return false
        }
        busy = true
        let failure = await Task.detached(priority: .userInitiated) { work(file) }.value
        busy = false
        message = failure?.message
        if let failure {
            await apply(Self.reaction(for: failure, step: loginStep))
            return false
        }
        if refreshAfter { await refresh() }
        return true
    }

    private func apply(_ reaction: AccountReaction) async {
        switch reaction {
        case .showMessage:
            break
        case .retryCode:
            code = ""
            codeFocusTick += 1
        case .lockEmail:
            code = ""
            loginStep = .email
            emailLocked = true
        case .signOutLocally:
            let keep = message
            await refresh()
            message = keep
        }
    }
}
