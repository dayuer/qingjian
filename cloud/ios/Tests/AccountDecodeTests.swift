// 账号页的纯逻辑：桥返回的 JSON 解码（形状与 qingjian-cloud-bridge 的 AccountStatus 一致）、Apple 登录的 nonce。

import XCTest
@testable import QingjianCloud

final class AccountDecodeTests: XCTestCase {
    func testSignedInStatusDecodes() throws {
        let json = """
        {"server":"https://pinyin.synon.ai","signed_in":true,
         "consents":{"clipboard":false,"sync":true,"input_log":false,"llm":true},
         "identities":[{"provider":"apple","label":null},{"provider":"email","label":"a@b.c"}],
         "sessions":[{"id":7,"name":"我的 iPhone","platform":"ios","created_at":1700000000000,"last_seen":null,"current":true},
                     {"id":8,"name":"MacBook","platform":"macos","created_at":1700000000000,"last_seen":1700000060000,"current":false}],
         "error":null}
        """
        let state = try XCTUnwrap(SettingsBridge.decode(json) as AccountState?)
        XCTAssertTrue(state.signedIn)
        XCTAssertEqual(state.consents, Consents(clipboard: false, sync: true, inputLog: false, llm: true))
        XCTAssertEqual(state.identities.map(\.title), ["Apple", "邮箱"])
        XCTAssertNil(state.identities[0].label)
        XCTAssertEqual(state.sessions.map(\.current), [true, false])
        XCTAssertEqual(state.sessions[1].lastSeen, 1_700_000_060_000)
        XCTAssertNil(state.error)
    }

    func testSignedOutStatusWithErrorDecodes() throws {
        let json = """
        {"server":"https://pinyin.synon.ai","signed_in":false,
         "consents":{"clipboard":false,"sync":false,"input_log":false,"llm":false},
         "identities":[],"sessions":[],"error":"登录已失效，请重新登录"}
        """
        let state = try XCTUnwrap(SettingsBridge.decode(json) as AccountState?)
        XCTAssertFalse(state.signedIn)
        XCTAssertEqual(state.error, "登录已失效，请重新登录")
    }

    func testBadJsonIsNil() {
        XCTAssertNil(SettingsBridge.decode("not json") as AccountState?)
        XCTAssertNil(SettingsBridge.decode(nil) as AccountState?)
    }

    func testConsentSubscriptAndFeatureNames() {
        var consents = Consents(clipboard: false, sync: false, inputLog: false, llm: false)
        consents[.inputLog] = true
        XCTAssertTrue(consents.inputLog)
        XCTAssertEqual(CloudFeature.allCases.map(\.rawValue), ["clipboard", "sync", "input_log", "llm"])
    }

    func testNonce() {
        XCTAssertEqual(
            Nonce.sha256Hex("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
        let first = Nonce.random()
        XCTAssertEqual(first.count, 64)
        XCTAssertNotEqual(first, Nonce.random())
    }
}

final class AccountFailureTests: XCTestCase {
    func testDecode() {
        XCTAssertNil(AccountFailure.decode(nil))
        let failure = AccountFailure.decode(#"{"code":"auth_failed","message":"验证码不对，请重新输入"}"#)
        XCTAssertEqual(failure, AccountFailure(code: .authFailed, message: "验证码不对，请重新输入"))
        let unknown = AccountFailure.decode(#"{"code":"brand_new","message":"x"}"#)
        XCTAssertEqual(unknown, AccountFailure(code: .other, message: "x"))
        let plain = AccountFailure.decode("连不上服务器")
        XCTAssertEqual(plain, AccountFailure(code: .other, message: "连不上服务器"))
    }

    func testStatusErrorCode() throws {
        func state(_ extra: String) throws -> AccountState {
            let json = """
            {"server":"s","signed_in":false,"consents":{"clipboard":false,"sync":false,"input_log":false,"llm":false},
             "identities":[],"sessions":[],"error":"e"\(extra)}
            """
            return try XCTUnwrap(SettingsBridge.decode(json) as AccountState?)
        }
        XCTAssertEqual(try state(#","error_code":"unauthorized""#).errorCode, .unauthorized)
        XCTAssertNil(try state("").errorCode)
        XCTAssertEqual(try state(#","error_code":"zzz""#).errorCode, .other)
    }

    func testReactions() {
        func reaction(_ code: AccountFailure.Code, _ step: LoginStep = .code) -> AccountReaction {
            AccountStore.reaction(for: AccountFailure(code: code, message: ""), step: step)
        }
        XCTAssertEqual(reaction(.authFailed, .code), .retryCode)
        XCTAssertEqual(reaction(.authFailed, .email), .showMessage)
        XCTAssertEqual(reaction(.lockedToday, .code), .lockEmail)
        XCTAssertEqual(reaction(.lockedToday, .email), .lockEmail)
        XCTAssertEqual(reaction(.unauthorized), .signOutLocally)
        for code: AccountFailure.Code in [
            .notConfigured, .rateLimited, .forbidden, .unreachable, .invalidArgument, .notSignedIn, .consentRequired, .other,
        ] {
            XCTAssertEqual(reaction(code), .showMessage, "\(code)")
        }
    }

    func testConsentRequiredDecodes() {
        let failure = AccountFailure.decode(#"{"code":"consent_required","message":"请先勾选同意，才能继续登录"}"#)
        XCTAssertEqual(failure?.code, .consentRequired)
        XCTAssertEqual(AccountFailure.Code.allCases.count, 11)
    }
}
