// 桥那几个成功返回的 JSON 怎么解：出码、输码的凭据、轮询三种状态、待处理申请，以及失败的 JSON 落到 BridgeFailure。

import XCTest
@testable import QingjianCloud

final class PairReplyTests: XCTestCase {
    private func decode<T: Decodable & Sendable>(_ json: String) -> T? {
        PairReply.decode(json)
    }

    /// 解错了的那个失败；解对了返回 nil。
    private func failure<T>(_ result: Result<T, BridgeFailure>) -> BridgeFailure? {
        if case .failure(let failure) = result { return failure }
        return nil
    }

    func testPairCodeUsesThePairCodeFieldNotTheFailureOne() {
        let reply: PairReply.Code? = decode(#"{"pair_code":"K7P2-9QXM","expires_at":1791043200000}"#)
        XCTAssertEqual(reply?.pairCode, "K7P2-9QXM")
        XCTAssertEqual(reply?.expiresAt, 1791043200000)

        // 失败 JSON 里也有 code，那份不该被解成出码
        let failure: Result<PairReply.Code, BridgeFailure> = PairReply.result(
            #"{"code":"bad_code","message":"匹配码不对或已经过期，请重新输一张"}"#)
        XCTAssertEqual(self.failure(failure)?.code, .badCode)
    }

    func testTicketCarriesTheSecretForPolling() {
        let reply: PairReply.Ticket? = decode(
            #"{"request_id":"r1","secret":"s1","expires_at":1791043200000}"#)
        XCTAssertEqual(reply?.requestId, "r1")
        XCTAssertEqual(reply?.secret, "s1")
        XCTAssertEqual(reply?.expiresAt, 1791043200000)
    }

    func testPollStatesDecodeAndUnknownOnesAreNotApproved() {
        let states = ["pending", "denied", "approved"].compactMap { raw -> PairReply.PollState? in
            let reply: PairReply.Poll? = decode(#"{"state":"\#(raw)"}"#)
            return reply?.state
        }
        XCTAssertEqual(states, [.pending, .denied, .approved])

        let unknown: PairReply.Poll? = decode(#"{"state":"who_knows"}"#)
        XCTAssertNil(unknown, "认不得的状态当解不出来，不许当成 approved")
    }

    func testRequestsDecodeAsAList() {
        let reply: [PairReply.Request]? = decode(
            #"[{"id":"r1","name":"新手机","platform":"ios","at":1791043200000}]"#)
        XCTAssertEqual(reply?.first?.name, "新手机")
        XCTAssertEqual(reply?.first?.id, "r1")
    }

    func testAFailureObjectIsAFailureNotAnEmptyList() {
        let reply: Result<[PairReply.Request], BridgeFailure> = PairReply.result(
            #"{"code":"not_signed_in","message":"还没有登录"}"#)
        XCTAssertEqual(failure(reply)?.code, .notSignedIn)
    }

    func testNothingBackIsAFailureNotACrash() {
        let reply: Result<PairReply.Code, BridgeFailure> = PairReply.result(nil)
        XCTAssertEqual(failure(reply)?.code, .other)
    }
}
