// 这两页上的话：失败说法跟着 BridgeFailure 的 code 走；界面不出现账号 / 登录 / 注册。

import XCTest
@testable import QingjianCloud

final class SpaceWordingTests: XCTestCase {
    func testTheTwoFailureKindsSayWhatToDo() {
        XCTAssertEqual(
            SpaceWording.failure(BridgeFailure(code: .badCode, message: "服务端的原文")),
            "匹配码不对或已经过期，请重新输一张")
        XCTAssertEqual(
            SpaceWording.failure(BridgeFailure(code: .deviceLimit, message: "服务端的原文")),
            "空间里的设备已经满了，先在旧设备上删一台再加")
        // 其余用桥给的中文
        XCTAssertEqual(
            SpaceWording.failure(BridgeFailure(code: .unreachable, message: "连不上服务器，检查网络后再试")),
            "连不上服务器，检查网络后再试")
    }

    func testNoScreenTextMentionsAccounts() {
        for text in SpaceWording.allTexts {
            for banned in ["账号", "登录", "注册"] {
                XCTAssertFalse(text.contains(banned), "「\(text)」里有「\(banned)」")
            }
        }
    }
}
