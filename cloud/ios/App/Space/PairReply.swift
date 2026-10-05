// 桥那六个空间接口的成功返回形状，与 `include/qingjian_bridge.h` 一一对应。
// 解不出来时试 `BridgeFailure`：失败是 `{"code","message"}`，成功那几种都没有 `message` 字段，两边不会撞。

import Foundation

enum PairReply {
    /// 出码的回复。字段叫 `pairCode` 而不是 `code`：失败那份 JSON 里 `code` 是错误种类。
    struct Code: Decodable, Equatable {
        let pairCode: String

        let expiresAt: Int64
    }

    /// 输码申请之后拿到的轮询凭据。
    struct Ticket: Decodable, Equatable {
        let requestId: String

        let secret: String

        let expiresAt: Int64
    }

    /// 轮询结果；`approved` 时令牌已经写进 `cloud.toml`，这里只有状态。
    enum PollState: String, Decodable, Equatable {
        case pending
        case denied
        case approved
    }

    struct Poll: Decodable, Equatable {
        let state: PollState
    }

    /// 等旧设备处理的申请。
    struct Request: Decodable, Equatable {
        let id: String

        let name: String

        let platform: String

        let at: Int64
    }

    /// 解一个成功的 JSON；解不出来返回 nil。
    static func decode<T: Decodable & Sendable>(_ json: String) -> T? {
        guard let data = json.data(using: .utf8) else { return nil }
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        return try? decoder.decode(T.self, from: data)
    }

    /// 桥回来的一个字符串：先按成功解，解不出来按失败解；两个都不像就是「看不懂的回应」。
    static func result<T: Decodable & Sendable>(_ json: String?) -> Result<T, BridgeFailure> {
        guard let json else {
            return .failure(BridgeFailure(code: .other, message: "桥没有返回内容"))
        }
        if let value: T = decode(json) {
            return .success(value)
        }
        if let failure = BridgeFailure.decode(json) {
            return .failure(failure)
        }
        return .failure(BridgeFailure(code: .other, message: json))
    }
}
