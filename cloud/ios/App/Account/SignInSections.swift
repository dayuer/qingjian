// 没登录时的两段：通过 Apple 登录、用邮箱登录（输入邮箱 → 发码 → 输入 6 位码）。

import AuthenticationServices
import SwiftUI

struct SignInSections: View {
    @Bindable var store: AccountStore

    @FocusState private var codeFocused: Bool

    @Environment(\.colorScheme) private var colorScheme

    private var canApple: Bool {
        AccountStore.canSignIn(consent: store.crossBorderConsent, busy: store.busy)
    }

    private var canEmail: Bool {
        AccountStore.canSignInWithEmail(
            consent: store.crossBorderConsent, busy: store.busy, locked: store.emailLocked)
    }

    var body: some View {
        Section {
            Toggle(isOn: $store.crossBorderConsent) {
                Text("同意将我的账号信息和我开启的云功能数据，存储在位于新加坡的服务器（腾讯云）并在那里处理，用于登录、同步与整理记忆。可随时在本页关闭功能或删除账号。")
                    .font(AppFont.footnote)
            }
            .toggleStyle(CheckboxToggleStyle())
            Link("了解更多", destination: PrivacyLinks.dataLocation)
                .font(AppFont.footnote)
        }
        Section {
            SignInWithAppleButton(.signIn) { request in
                store.prepare(request)
            } onCompletion: { result in
                Task { await store.completeApple(result) }
            }
            .signInWithAppleButtonStyle(colorScheme == .dark ? .white : .black)
            .frame(height: 44)
            .disabled(!canApple)
            .opacity(canApple ? 1 : 0.35)
        } header: {
            Text("登录")
        } footer: {
            Text("登录后可以在 iPhone 与 Mac 之间同步剪贴板、输入习惯，使用大模型润色。各项功能默认关闭，登录后逐项打开。")
        }
        Section {
            if store.loginStep == .code {
                TextField("6 位验证码", text: $store.code)
                    .keyboardType(.numberPad)
                    .textContentType(.oneTimeCode)
                    .focused($codeFocused)
                Button("登录") {
                    Task { await store.emailVerify(store.email, code: store.code) }
                }
                .disabled(store.code.count != 6 || !canEmail)
                Button("换一个邮箱") {
                    store.code = ""
                    store.loginStep = .email
                }
            } else {
                TextField("邮箱地址", text: $store.email)
                    .keyboardType(.emailAddress)
                    .textContentType(.emailAddress)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                Button("发送验证码") {
                    Task { await store.emailStart(store.email) }
                }
                .disabled(!AccountStore.normalized(email: store.email).contains("@") || !canEmail)
            }
        } header: {
            Text("用邮箱登录")
        } footer: {
            if store.emailLocked {
                Text("今天不能再用邮箱验证码登录，请用上面的 Apple 登录。")
            } else {
                Text(store.loginStep == .code ? "验证码已发到 \(store.email)。" : "第一次用这个邮箱登录会自动注册，不需要密码。")
            }
        }
        .disabled(store.emailLocked)
        .opacity(store.emailLocked ? 0.4 : 1)
        .onChange(of: store.codeFocusTick) { codeFocused = true }
    }
}
