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

    /// 用户是否勾选了「同意把数据存到境外服务器」。不持久化：每次进登录页、退出登录后都要重新勾。
    var crossBorderConsent = false

    /// 当天验证失败太多次被锁：邮箱区域禁用，只能用 Apple 登录。
    private(set) var emailLocked = false

    /// 每加一次，验证码输入框抢一次焦点。
    private(set) var codeFocusTick = 0

    /// 这一次 Apple 登录的原始 nonce；请求里给 Apple 的是它的 SHA-256。
    private var appleNonce: String?

    private static let noGroup = "这个安装包没有开通 App Group，账号用不了"

    /// 页面出现时重取一遍；已有操作在跑（它自己会重取）就不再并发，免得后写覆盖先写。
    func refresh() async {
        guard Self.mayEnter(busy: busy) else { return }
        busy = true
        defer { busy = false }
        await reload()
    }

    /// 是否允许开始一次新的桥调用：busy 时不允许，连点只提交一次。
    nonisolated static func mayEnter(busy: Bool) -> Bool { !busy }

    /// 能否发起 Apple 登录：要勾了同意，且没有别的操作在跑。
    nonisolated static func canSignIn(consent: Bool, busy: Bool) -> Bool { consent && !busy }

    /// 能否发起邮箱登录：同上，并且当天没被锁。
    nonisolated static func canSignInWithEmail(consent: Bool, busy: Bool, locked: Bool) -> Bool {
        canSignIn(consent: consent, busy: busy) && !locked
    }

    /// 邮箱先去掉首尾空白再校验、再发给桥。
    nonisolated static func normalized(email: String) -> String {
        email.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    /// 账号文件位置；测试里换成临时目录。
    @ObservationIgnored var fileProvider: () -> URL? = { SharedStore.cloudFile }

    private func reload() async {
        guard let file = fileProvider() else {
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
    nonisolated static func reaction(for failure: BridgeFailure, step: LoginStep) -> AccountReaction {
        switch failure.code {
        case .authFailed: step == .code ? .retryCode : .showMessage
        case .lockedToday: .lockEmail
        case .unauthorized: .signOutLocally
        // 空间那条路的两种说法在 App/Space 里处理（那边自己认 code），账号这页只把原文显示出来
        case .notConfigured, .rateLimited, .forbidden, .unreachable, .invalidArgument, .notSignedIn,
             .consentRequired, .badCode, .deviceLimit, .other:
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
            appleNonce = nil
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
            appleNonce = nil
            message = "Apple 没有给出登录凭据，请重试"
            return
        }
        appleNonce = nil
        let device = UIDevice.current.name
        let consented = crossBorderConsent
        if await perform({
            AccountBridge.signInApple(
                $0, identityToken: token, authorizationCode: code, nonce: nonce, device: device,
                crossBorderConsented: consented)
        }) {
            message = "已登录"
        }
    }

    /// 发验证码；成功返回 true。
    func emailStart(_ rawEmail: String) async -> Bool {
        let email = Self.normalized(email: rawEmail)
        let consented = crossBorderConsent
        let sent = await perform(
            { AccountBridge.emailStart($0, email: email, crossBorderConsented: consented) }, refreshAfter: false)
        if sent {
            loginStep = .code
            message = "验证码已发出，10 分钟内有效"
        }
        return sent
    }

    /// 用验证码登录；成功返回 true。
    func emailVerify(_ rawEmail: String, code: String) async -> Bool {
        let email = Self.normalized(email: rawEmail)
        let consented = crossBorderConsent
        let device = UIDevice.current.name
        let signedIn = await perform({
            AccountBridge.emailVerify(
                $0, email: email, code: code, device: device, crossBorderConsented: consented)
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

    /// 清空云端输入记录，连带删本机日志与上传进度。
    func clearInputLog() async -> Bool {
        guard let userDir = SharedStore.directory else {
            message = "还没有可清的记录"
            return false
        }
        let dir = userDir
        let ok = await perform { file in
            AccountBridge.clearInputLog(cloudFile: file, userDir: dir)
        }
        if !ok { message = "清空失败，稍后再试" }
        return ok
    }

    func revoke(_ device: AccountDevice) async {
        let id = device.id
        if await perform({ AccountBridge.revokeSession($0, id: id) }) {
            message = "已解绑「\(device.name)」"
        }
    }

    func signOut() async {
        if await perform({ AccountBridge.signOut($0) }) {
            crossBorderConsent = false
            message = "已退出登录"
        }
    }

    func deleteAccount() async {
        if await perform({ AccountBridge.deleteAccount($0) }) {
            message = "账号已删除，服务器上的数据已清除"
        }
    }

    /// 在后台跑一次桥的操作；失败时按 code 处理并显示 message。成功后默认整页重取。
    @discardableResult
    func perform(
        _ work: @escaping @Sendable (URL) -> BridgeFailure?, refreshAfter: Bool = true
    ) async -> Bool {
        guard Self.mayEnter(busy: busy) else { return false }
        guard let file = fileProvider() else {
            message = Self.noGroup
            return false
        }
        busy = true
        defer { busy = false }
        let failure = await Task.detached(priority: .userInitiated) { work(file) }.value
        message = failure?.message
        if let failure {
            await apply(Self.reaction(for: failure, step: loginStep))
            return false
        }
        if refreshAfter { await reload() }
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
            crossBorderConsent = false
            let keep = message
            await reload()
            message = keep
        }
    }
}
