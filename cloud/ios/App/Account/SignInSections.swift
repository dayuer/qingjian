// 没登录时的两段：通过 Apple 登录、用邮箱登录（输入邮箱 → 发码 → 输入 6 位码）。

import AuthenticationServices
import SwiftUI

struct SignInSections: View {
    let store: AccountStore

    @State private var email = ""

    @State private var code = ""

    @State private var codeSent = false

    @Environment(\.colorScheme) private var colorScheme

    var body: some View {
        Section {
            SignInWithAppleButton(.signIn) { request in
                store.prepare(request)
            } onCompletion: { result in
                Task { await store.completeApple(result) }
            }
            .signInWithAppleButtonStyle(colorScheme == .dark ? .white : .black)
            .frame(height: 44)
        } header: {
            Text("登录")
        } footer: {
            Text("登录后可以在 iPhone 与 Mac 之间同步剪贴板、输入习惯，使用大模型润色。各项功能默认关闭，登录后逐项打开。")
        }
        Section {
            if codeSent {
                TextField("6 位验证码", text: $code)
                    .keyboardType(.numberPad)
                    .textContentType(.oneTimeCode)
                Button("登录") {
                    Task {
                        if await store.emailVerify(email, code: code) {
                            code = ""
                            codeSent = false
                        }
                    }
                }
                .disabled(code.count != 6)
                Button("换一个邮箱") {
                    code = ""
                    codeSent = false
                }
            } else {
                TextField("邮箱地址", text: $email)
                    .keyboardType(.emailAddress)
                    .textContentType(.emailAddress)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                Button("发送验证码") {
                    Task { codeSent = await store.emailStart(email) }
                }
                .disabled(!email.contains("@"))
            }
        } header: {
            Text("用邮箱登录")
        } footer: {
            Text(codeSent ? "验证码已发到 \(email)。" : "第一次用这个邮箱登录会自动注册，不需要密码。")
        }
    }
}
