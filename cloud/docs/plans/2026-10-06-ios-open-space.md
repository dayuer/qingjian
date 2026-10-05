# iOS 开通云服务与新设备（1b Task 3）实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** iOS 上「不要账号」这条路能走通：引导第三步点「了解云服务」或「我」页点「素笺云服务」，勾一次出境同意就把空间建起来，这台设备当场成为已登录设备；新设备输一张 8 位匹配码申请加入，旧设备允许后就同样登录。

**Architecture:** 桥的六个 C 接口（`qj_space_create` / `qj_pair_*`）与令牌落盘已经在 `impl/no-account-client` 上做完，这一件只动 iOS 显示层：一个薄的桥封装 `PairBridge`、几个可测的纯值（`MatchCode` / `SpaceFlow` / `SpaceWording`）、一个 `@Observable` 的 `SpaceStore`、两页 SwiftUI（开通 / 输码），再接进「我」页与首次引导。

**Tech Stack:** Swift 6 / SwiftUI（iOS 17）、XcodeGen（`cloud/ios/project.yml`）、XCTest（`cloud/ios/Tests`）；Rust 桥加一个本地判断的小接口。

**上游材料：** [不要账号客户端计划](2026-10-05-no-account-client.md)、[UI 清单](2026-10-05-ui-implementation.md)（屏 05 的 2e / 2f / 2j）、设计稿快照 [`cloud/design/mockups/05 记录与免费版.dc.html`](../../design/mockups/05%20记录与免费版.dc.html)。

---

## 这一轮的范围（先读这一段）

上游计划 Task 3 的原文是「2e「了解云服务」→ 出境同意 → 建空间 → 2f；新设备「输入匹配码 / 用找回方式」；去掉原「账号」页的登录入口」。
按决定 D7（缩范围）与 D6（设计稿缺屏）切掉四块，**都不在本计划里**：

| 切掉的 | 为什么 | 留给谁 |
|---|---|---|
| 「用找回方式」加入（Apple / 邮箱 / 微信） | D7：微信不做，绑定和找回往后放；服务端这一轮也没有 | 下一轮 |
| 出码页（旧设备出示匹配码） | 属于 Task 4「设备与找回页」 | 1b Task 4 |
| 2f「开启记录」同意页 | 是 2B Task 6 的一屏；它挂在开通流程的**下游**，本轮建完空间就停 | 2B Task 6 |
| 订阅与「下次续费」（2j 那半行） | 走苹果内购，定价未定 | 后面 |

**D6 未答**（设计稿没画设备页 / 匹配码输入页 / 出码页）。UI 清单给的备选是「或明确照系统表单样式做」，本计划按这一条走：
两页都是普通的 `Form`（分组列表 + 系统 `TextField` / `Button`），**交 UI 审计员过**。设计稿补稿后按稿改版式，逻辑不用动。

**出境同意的文案要改一句。** 现在那句话在 `SignInSections.swift`，开头是「同意将我的**账号信息**和我开启的云功能数据……」——
这一版没有账号了，得改成不提账号的说法。本计划先按下面这句写，**维护者定稿后替换**（`SpaceWording.consent`，一处）：

> 同意把我开启的云功能数据存在位于新加坡的服务器（腾讯云）并在那里处理，用于同步与整理记忆。可随时在「我」里关掉功能或删掉云端数据。

---

## 文件结构

| 路径 | 职责 |
|---|---|
| `cloud/crates/qingjian-cloud-bridge/src/lib.rs` | 加 `qj_cloud_signed_in`（只读 `cloud.toml`，不联网） |
| `cloud/crates/qingjian-cloud-bridge/include/qingjian_bridge.h` | 同步声明 |
| `cloud/crates/qingjian-cloud-bridge/tests/cloud_signed_in_ffi.rs` | 那个接口的测试 |
| `cloud/ios/App/Account/BridgeFailure.swift` | 由 `AccountFailure.swift` 改名：桥的失败 JSON（账号与空间共用） |
| `cloud/ios/App/Space/PairBridge.swift` | 调桥的六个接口 + `signedIn()`，JSON 进 JSON 出 |
| `cloud/ios/App/Space/PairReply.swift` | 桥那几种成功返回的形状（`Ticket` / `Code` / `Poll` / `Request`） |
| `cloud/ios/App/Space/MatchCode.swift` | 纯值：用户输入 → 规范化后的码，够不够 8 位 |
| `cloud/ios/App/Space/SpaceFlow.swift` | 纯值：轮询该不该继续、还要等多久 |
| `cloud/ios/App/Space/SpaceWording.swift` | 两页与各种失败的全部文案 |
| `cloud/ios/App/Space/SpaceStore.swift` | `@Observable`：开通、输码、轮询、取消 |
| `cloud/ios/App/Space/CreateSpaceView.swift` | 开通页：说明 + 出境同意 + 建空间 |
| `cloud/ios/App/Space/JoinSpaceView.swift` | 新设备：输码 → 等允许 → 结果 |
| `cloud/ios/App/Me/MeView.swift` | 加「素笺云服务」一行 |
| `cloud/ios/App/Onboarding/PlanStep.swift`、`OnboardingView.swift` | 「了解云服务」从说明页改成开通页 |
| `cloud/ios/Tests/` | `PairReplyTests` / `MatchCodeTests` / `SpaceFlowTests` / `SpaceWordingTests` / `BridgeFailureTests` |

---

## Task 1：`AccountFailure` 改名 `BridgeFailure`，补两个 code

**为什么：** 这个类型是**整个桥**的失败 JSON（`{"code","message"}`），不只账号用；空间这一路要用它，不改名的话
`App/Space/` 里出现 `AccountFailure` 会让人以为还没去掉账号。两个新 code 与桥的 `failure.rs` 对齐（`impl/no-account-client` 加的）。

**Files:**
- Rename: `cloud/ios/App/Account/AccountFailure.swift` → `cloud/ios/App/Account/BridgeFailure.swift`
- Modify: `AccountBridge.swift`、`AccountState.swift`、`AccountStore.swift`、`Tests/AccountDecodeTests.swift`、`Tests/AccountStoreTests.swift`

- [ ] **Step 1：改名**

```bash
git mv cloud/ios/App/Account/AccountFailure.swift cloud/ios/App/Account/BridgeFailure.swift
grep -rl "AccountFailure" cloud/ios/App cloud/ios/Tests \
  | xargs sed -i '' 's/AccountFailure/BridgeFailure/g'
```

`BridgeFailure.swift` 的文件头第一行改准：

```swift
// 桥的 `qj_*` 操作失败时返回的 JSON：`{"code": "...", "message": "..."}`。message 是给用户看的中文，原样显示；code 给界面分支用。
// 账号、空间、记忆都用这一份（记忆那套另有一套 code，走 `qj_memory_*`）。
```

- [ ] **Step 2：加两个 code**

`BridgeFailure.Code` 里 `case notSignedIn = "not_signed_in"` 之后插：

```swift
        case badCode = "bad_code"
        case deviceLimit = "device_limit"
```

- [ ] **Step 3：加测试**

`Tests/AccountDecodeTests.swift` 的 `Code` 往返用例里补两条（与既有写法一致）：

```swift
    func testSpaceCodesDecode() {
        XCTAssertEqual(BridgeFailure.Code(rawValue: "bad_code"), .badCode)
        XCTAssertEqual(BridgeFailure.Code(rawValue: "device_limit"), .deviceLimit)
        // 认不得的仍然是 other
        XCTAssertEqual(BridgeFailure.Code(rawValue: "who_knows"), .other)
    }
```

- [ ] **Step 4：跑**

```bash
cd cloud/ios && xcodebuild -project QingjianCloud.xcodeproj -scheme QingjianCloud \
  -destination 'platform=iOS Simulator,name=iPhone 17' -only-testing:QingjianCloudTests test
```

预期：与改名前同样的用例数、全过。

- [ ] **Step 5：提交**

```bash
git add cloud/ios
git commit -m "refactor(ios): 失败 JSON 的类型改名 BridgeFailure，补空间的两个 code"
```

---

## Task 2：桥加 `qj_cloud_signed_in`

**为什么：**「我」页那一行要显示「已开通 / 没开通」，而 `qj_account_status` 在已登录时会**联网**——
每次进「我」页都发一次请求不合适。这一个只读 `cloud.toml` 判断令牌前缀，不联网。

**Files:**
- Modify: `cloud/crates/qingjian-cloud-bridge/src/lib.rs`
- Modify: `cloud/crates/qingjian-cloud-bridge/include/qingjian_bridge.h`
- Test: `cloud/crates/qingjian-cloud-bridge/tests/cloud_signed_in_ffi.rs`

- [ ] **Step 1：写失败的测试**

```rust
//! `qj_cloud_signed_in`：只看 `cloud.toml` 里的令牌，不联网。

use std::ffi::{CString, c_char};

use qingjian_cloud_bridge as _;

unsafe extern "C" {
    fn qj_cloud_signed_in(path: *const c_char) -> bool;
}

fn cloud_toml(name: &str, text: &str) -> CString {
    let dir = std::env::temp_dir().join(format!("qj-signed-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("cloud.toml");
    std::fs::write(&path, text).unwrap();
    CString::new(path.to_str().unwrap()).unwrap()
}

#[test]
fn a_session_token_means_signed_in() {
    let path = cloud_toml("token", "server = \"http://127.0.0.1:1\"\ntoken = \"sjt_t\"\n");
    assert!(unsafe { qj_cloud_signed_in(path.as_ptr()) });
}

#[test]
fn no_token_a_legacy_token_or_a_broken_file_means_signed_out() {
    for (name, text) in [
        ("none", "server = \"s\"\n"),
        ("legacy", "server = \"s\"\ntoken = \"qjc_old\"\n"),
        ("broken", "这不是 toml ]"),
    ] {
        let path = cloud_toml(name, text);
        assert!(!unsafe { qj_cloud_signed_in(path.as_ptr()) }, "{name}");
    }
}

#[test]
fn a_missing_file_or_a_null_path_is_signed_out_not_a_crash() {
    let missing = CString::new("/nonexistent-qj-signed/cloud.toml").unwrap();
    assert!(!unsafe { qj_cloud_signed_in(missing.as_ptr()) });
    assert!(!unsafe { qj_cloud_signed_in(std::ptr::null()) });
}
```

- [ ] **Step 2：跑，确认失败**

```bash
cargo test --manifest-path cloud/Cargo.toml -p qingjian-cloud-bridge --test cloud_signed_in_ffi
```

预期：编译不过，`cannot find function qj_cloud_signed_in`。

- [ ] **Step 3：实现**

`lib.rs` 的 `qj_settings_*` 那一段（有会话的接口附近）之后加：

```rust
/// 本机有没有拿到过会话（只读 `cloud.toml`，不联网）：App 用它在「我」页显示「已开通 / 没开通」。
///
/// # Safety
/// `path` 是有效的 UTF-8 C 字符串或空指针。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_cloud_signed_in(path: *const c_char) -> bool {
    unsafe { path_arg(path) }
        .is_some_and(|path| CloudConfig::read(Path::new(path)).is_some_and(|c| c.signed_in()))
}
```

- [ ] **Step 4：同步头文件**

`qingjian_bridge.h` 的账号那一段开头（`char *qj_account_status(const char *path);` 之前）加：

```c
// 本机有没有拿到过会话：只读 cloud.toml 的令牌，不联网。App 用它决定「我」页那一行显示已开通还是没开通。
bool qj_cloud_signed_in(const char *path);
```

- [ ] **Step 5：跑**

```bash
cargo test --manifest-path cloud/Cargo.toml -p qingjian-cloud-bridge
```

预期：全过。

- [ ] **Step 6：提交**

```bash
git add cloud/crates/qingjian-cloud-bridge
git commit -m "feat(cloud): 桥加一个只看本机 cloud.toml 的已登录判断"
```

---

## Task 3：`PairReply` 与 `PairBridge`

**为什么：** Swift 侧要有一个地方把桥的六个接口包起来，并把返回的 JSON 解成类型。JSON 的解法要能单测，
所以「解」与「调」分开：`PairReply` 是纯解码，`PairBridge` 负责传字符串。

**Files:**
- Create: `cloud/ios/App/Space/PairReply.swift`
- Create: `cloud/ios/App/Space/PairBridge.swift`
- Test: `cloud/ios/Tests/PairReplyTests.swift`

- [ ] **Step 1：写失败的测试**

```swift
// 桥那几个成功返回的 JSON 怎么解：出码、输码的凭据、轮询三种状态、待处理申请，以及失败的 JSON 落到 BridgeFailure。

import XCTest
@testable import QingjianCloud

final class PairReplyTests: XCTestCase {
    private func decode<T: Decodable>(_ json: String) -> T? {
        PairReply.decode(json)
    }

    func testPairCodeUsesThePairCodeFieldNotTheFailureOne() {
        let reply: PairReply.Code? = decode(#"{"pair_code":"K7P2-9QXM","expires_at":1791043200000}"#)
        XCTAssertEqual(reply?.pairCode, "K7P2-9QXM")
        XCTAssertEqual(reply?.expiresAt, 1791043200000)

        // 失败 JSON 里也有 code，那份不该被解成出码
        let failure: Result<PairReply.Code, BridgeFailure> = PairReply.result(
            #"{"code":"bad_code","message":"匹配码不对或已经过期，请重新输一张"}"#)
        XCTAssertEqual(failure.failure?.code, .badCode)
    }

    func testTicketCarriesTheSecretForPolling() {
        let reply: PairReply.Ticket? = decode(
            #"{"request_id":"r1","secret":"s1","expires_at":1791043200000}"#)
        XCTAssertEqual(reply?.requestId, "r1")
        XCTAssertEqual(reply?.secret, "s1")
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
        XCTAssertEqual(reply.failure?.code, .notSignedIn)
    }
}
```

- [ ] **Step 2：跑，确认失败**

```bash
cd cloud/ios && xcodebuild -project QingjianCloud.xcodeproj -scheme QingjianCloud \
  -destination 'platform=iOS Simulator,name=iPhone 17' \
  -only-testing:QingjianCloudTests/PairReplyTests test
```

预期：编译失败，`cannot find 'PairReply' in scope`。

- [ ] **Step 3：写 `PairReply`**

```swift
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
    static func decode<T: Decodable>(_ json: String) -> T? {
        guard let data = json.data(using: .utf8) else { return nil }
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        return try? decoder.decode(T.self, from: data)
    }

    /// 桥回来的一个字符串：先按成功解，解不出来按失败解；两个都不像就是「看不懂的回应」。
    static func result<T: Decodable>(_ json: String?) -> Result<T, BridgeFailure> {
        guard let json else { return .failure(BridgeFailure(code: .other, message: "桥没有返回内容")) }
        if let value: T = decode(json) {
            return .success(value)
        }
        if let failure = BridgeFailure.decode(json) {
            return .failure(failure)
        }
        return .failure(BridgeFailure(code: .other, message: json))
    }
}
```

- [ ] **Step 4：写 `PairBridge`**

```swift
// 调 Rust 桥的空间接口（qj_space_* / qj_pair_*）：都是阻塞的网络请求，只在后台任务里调（见 SpaceStore）。
// 令牌留在桥与 cloud.toml 之间，不经过 Swift。

import Foundation
import QingjianBridge

enum PairBridge {
    /// 建空间；成功这台设备就已登录（令牌写进 cloud.toml）。成功返回 nil，失败返回原因。
    static func createSpace(_ file: URL, device: String, crossBorderConsented: Bool) -> BridgeFailure? {
        failure(file.path.withCString { f in
            device.withCString { d in qj_space_create(f, d, crossBorderConsented) }
        })
    }

    static func pairCode(_ file: URL) -> Result<PairReply.Code, BridgeFailure> {
        PairReply.result(take(file.path.withCString { qj_pair_code($0) }))
    }

    static func pairJoin(_ file: URL, code: String, device: String) -> Result<PairReply.Ticket, BridgeFailure> {
        PairReply.result(take(file.path.withCString { f in
            code.withCString { c in device.withCString { d in qj_pair_join(f, c, d) } }
        }))
    }

    static func pairPoll(_ file: URL, requestId: String, secret: String) -> Result<PairReply.Poll, BridgeFailure> {
        PairReply.result(take(file.path.withCString { f in
            requestId.withCString { r in secret.withCString { s in qj_pair_poll(f, r, s) } }
        }))
    }

    static func pairRequests(_ file: URL) -> Result<[PairReply.Request], BridgeFailure> {
        PairReply.result(take(file.path.withCString { qj_pair_requests($0) }))
    }

    static func pairDecide(_ file: URL, requestId: String, allow: Bool) -> BridgeFailure? {
        failure(file.path.withCString { f in
            requestId.withCString { r in qj_pair_decide(f, r, allow) }
        })
    }

    /// 本机有没有拿到过会话（只看 cloud.toml，不联网）。
    static func signedIn(_ file: URL) -> Bool {
        file.path.withCString { qj_cloud_signed_in($0) }
    }

    /// NULL 是成功。
    private static func failure(_ raw: UnsafeMutablePointer<CChar>?) -> BridgeFailure? {
        BridgeFailure.decode(SettingsBridge.take(raw))
    }

    private static func take(_ raw: UnsafeMutablePointer<CChar>?) -> String? {
        SettingsBridge.take(raw)
    }
}
```

- [ ] **Step 5：把两个新文件加进工程并跑**

`PairReply.swift`、`PairBridge.swift` 在 `App/` 下，`project.yml` 按目录收，不用改。但 `Tests/` 同理。跑：

```bash
cd cloud/ios && xcodegen generate && xcodebuild -project QingjianCloud.xcodeproj -scheme QingjianCloud \
  -destination 'platform=iOS Simulator,name=iPhone 17' \
  -only-testing:QingjianCloudTests/PairReplyTests test
```

预期：5 条全过。

- [ ] **Step 6：提交**

```bash
git add cloud/ios/App/Space cloud/ios/Tests/PairReplyTests.swift cloud/ios/QingjianCloud.xcodeproj 2>/dev/null
git add cloud/ios/App/Space cloud/ios/Tests/PairReplyTests.swift
git commit -m "feat(ios): 包一层桥的空间接口，解它的几种回复"
```

---

## Task 4：`MatchCode`（纯值）

**为什么：** 用户看到的码是 `K7P2-9QXM`，粘进来常带空格或连字符，还可能带大小写。本地只做「去空白与连字符 + 转大写 + 数位数」
用于即时反馈（还差几位），**不做 Crockford 的 `I/L→1`、`O→0` 映射**——那是服务端 `normalize_pair_code` 的事，
本地映射一份就是第二份真相（映射错了还是服务端报 `bad_code`）。

**Files:**
- Create: `cloud/ios/App/Space/MatchCode.swift`
- Test: `cloud/ios/Tests/MatchCodeTests.swift`

- [ ] **Step 1：写失败的测试**

```swift
// 用户输入的匹配码怎么算：去掉空白与连字符、转大写；8 位才算输完。

import XCTest
@testable import QingjianCloud

final class MatchCodeTests: XCTestCase {
    func testWhitespaceAndHyphensAreDroppedAndLettersGoUpper() {
        XCTAssertEqual(MatchCode.entered(" k7p2-9qxm "), "K7P29QXM")
        XCTAssertEqual(MatchCode.entered("K7P2 9QXM"), "K7P29QXM")
    }

    func testOnlyEightCharactersCountAsComplete() {
        XCTAssertFalse(MatchCode.isComplete("K7P29QX"))
        XCTAssertTrue(MatchCode.isComplete("K7P29QXM"))
        XCTAssertFalse(MatchCode.isComplete(""))
        // 连字符与空白不算位数
        XCTAssertTrue(MatchCode.isComplete("K7P2-9QXM"))
    }

    func testTheRemainingCountTellsHowManyKeysAreMissing() {
        XCTAssertEqual(MatchCode.remaining("K7P2-9QXM"), 0)
        XCTAssertEqual(MatchCode.remaining("K7P2-9"), 3)
        XCTAssertEqual(MatchCode.remaining(""), 8)
    }

    func testAnythingLongerIsNotSilentlyTrimmed() {
        // 多打的字符留着：让服务端去判，本地裁掉会掩盖用户粘错东西
        XCTAssertEqual(MatchCode.entered("K7P2-9QXM123"), "K7P29QXM123")
        XCTAssertFalse(MatchCode.isComplete("K7P2-9QXM123"))
    }
}
```

- [ ] **Step 2：跑，确认失败**

```bash
cd cloud/ios && xcodebuild -project QingjianCloud.xcodeproj -scheme QingjianCloud \
  -destination 'platform=iOS Simulator,name=iPhone 17' -only-testing:QingjianCloudTests/MatchCodeTests test
```

预期：编译失败，`cannot find 'MatchCode' in scope`。

- [ ] **Step 3：实现**

```swift
// 用户输入的匹配码：去掉空白与连字符、转大写，够不够 8 位。
// 只做这一层，`I/L→1`、`O→0` 那些 Crockford 的规范化放在服务端（proto 的 `normalize_pair_code`），不做第二份。

import Foundation

enum MatchCode {
    /// 匹配码的位数，与 `qingjian_cloud_proto::PAIR_CODE_LEN` 一致。
    static let length = 8

    /// 去掉空白与连字符、转成大写；再长的原样留着（让服务端去判）。
    static func entered(_ raw: String) -> String {
        raw.filter { !$0.isWhitespace && $0 != "-" }.uppercased()
    }

    static func isComplete(_ raw: String) -> Bool {
        entered(raw).count == length
    }

    /// 还差几位；输多了是 0，不是负数。
    static func remaining(_ raw: String) -> Int {
        max(0, length - entered(raw).count)
    }
}
```

- [ ] **Step 4：跑**

预期：4 条全过。

- [ ] **Step 5：提交**

```bash
git add cloud/ios/App/Space/MatchCode.swift cloud/ios/Tests/MatchCodeTests.swift
git commit -m "feat(ios): 匹配码在本地只做去空白与数位数"
```

---

## Task 5：`SpaceFlow`（纯值）与 `SpaceWording`（文案）

**为什么：** 轮询「等旧设备允许」是个循环，得有明确的边界（多久一次、等到什么时候停），做成纯值才好测；
文案集中一处，UI 清单里「界面不出现账号、登录、注册」那条也要有个地方能被测试锁住。

**Files:**
- Create: `cloud/ios/App/Space/SpaceFlow.swift`
- Create: `cloud/ios/App/Space/SpaceWording.swift`
- Test: `cloud/ios/Tests/SpaceFlowTests.swift`
- Test: `cloud/ios/Tests/SpaceWordingTests.swift`

- [ ] **Step 1：写失败的测试**

`crates/qingjian-cloud-bridge/tests/` 那种风格，Swift 版：

```swift
// 等旧设备允许的那个循环：多久问一次、什么时候停。

import XCTest
@testable import QingjianCloud

final class SpaceFlowTests: XCTestCase {
    private let now: Int64 = 1_791_043_200_000

    func testItPollsEveryTwoSecondsWhileTheTicketIsAlive() {
        let flow = SpaceFlow(expiresAt: now + 60_000)
        XCTAssertTrue(flow.shouldPoll(now: now))
        XCTAssertEqual(flow.nextDelay(now: now), 2.0)
    }

    func testItStopsWhenTheTicketExpires() {
        let flow = SpaceFlow(expiresAt: now + 60_000)
        XCTAssertFalse(flow.shouldPoll(now: now + 60_000), "到点就停，不再问")
        XCTAssertFalse(flow.shouldPoll(now: now + 60_001))
    }

    func testAnAlreadyExpiredTicketNeverPolls() {
        let flow = SpaceFlow(expiresAt: now)
        XCTAssertFalse(flow.shouldPoll(now: now))
    }

    func testTheLastPollBeforeExpiryWaitsShorterRatherThanOverrunning() {
        // 还剩 0.5 秒：等 0.5 秒就问最后一次取到结果，而不是干等 2 秒、过了期还不知道
        let flow = SpaceFlow(expiresAt: now + 500)
        XCTAssertEqual(flow.nextDelay(now: now), 0.5, accuracy: 0.001)
    }

    func testTheDelayIsNeverZeroOrNegative() {
        let flow = SpaceFlow(expiresAt: now + 1)
        XCTAssertGreaterThan(flow.nextDelay(now: now), 0)
    }
}
```

```swift
// 这两页上的话：失败说法跟着 BridgeFailure 的 code 走；界面不出现账号 / 登录 / 注册。

import XCTest
@testable import QingjianCloud

final class SpaceWordingTests: XCTestCase {
    func testTheTwoFailureKindsSayWhatToDo() {
        XCTAssertEqual(
            SpaceWording.failure(.init(code: .badCode, message: "服务端的原文")),
            "匹配码不对或已经过期，请重新输一张")
        XCTAssertEqual(
            SpaceWording.failure(.init(code: .deviceLimit, message: "服务端的原文")),
            "空间里的设备已经满了，先在旧设备上删一台再加")
        // 其余用桥给的中文
        XCTAssertEqual(
            SpaceWording.failure(.init(code: .unreachable, message: "连不上服务器，检查网络后再试")),
            "连不上服务器，检查网络后再试")
    }

    func testNoScreenTextMentionsAccounts() {
        for text in SpaceWording.allTexts {
            for banned in ["账号", "登录", "注册"] {
                XCTAssertFalse(text.contains(banned), "「\(text)」里有「\(banned)」")
            }
        }
    }

    func testTheWaitingTextCountsWhatIsHappening() {
        XCTAssertEqual(SpaceWording.waiting, "等另一台设备允许…")
        XCTAssertEqual(SpaceWording.denied, "另一台设备没有允许这次加入，可以让它再出一张码")
    }
}
```

- [ ] **Step 2：跑，确认失败**

```bash
cd cloud/ios && xcodebuild -project QingjianCloud.xcodeproj -scheme QingjianCloud \
  -destination 'platform=iOS Simulator,name=iPhone 17' -only-testing:QingjianCloudTests/SpaceFlowTests test
```

预期：编译失败，`cannot find 'SpaceFlow' in scope`。

- [ ] **Step 3：实现 `SpaceFlow`**

```swift
// 输码之后等旧设备允许：多久问一次、什么时候停。到 `expires_at` 就停，不再问——那张申请在服务端已经不作数了。

import Foundation

struct SpaceFlow: Equatable {
    /// 每两次轮询之间等多久。
    static let interval: TimeInterval = 2

    /// 这次申请的过期时间，Unix 毫秒（桥给的 `expires_at`）。
    let expiresAt: Int64

    func shouldPoll(now: Int64) -> Bool {
        now < expiresAt
    }

    /// 下一次轮询前等多久：不会越过 `expires_at`，也不会是 0。
    func nextDelay(now: Int64) -> TimeInterval {
        let left = TimeInterval(expiresAt - now) / 1000
        return max(0.2, min(Self.interval, left))
    }
}
```

- [ ] **Step 4：实现 `SpaceWording`**

```swift
// 开通页与输码页上的全部文字。集中一处：UI 清单约束 5 要求界面不出现「账号 / 登录 / 注册」，
// 那条由 `allTexts` 的测试守着。

import Foundation

enum SpaceWording {
    /// 出境同意那一句。**维护者定稿后替换这里**（原话在 `SignInSections.swift`，开头是「我的账号信息」，这一版没有账号了）。
    static let consent = "同意把我开启的云功能数据存在位于新加坡的服务器（腾讯云）并在那里处理，用于同步与整理记忆。可随时在「我」里关掉功能或删掉云端数据。"

    static let createTitle = "开通素笺云服务"

    static let createIntro = "开通后在键盘上记的事会存到云端，每天整理成记忆卡，换手机也还在。"

    static let createButton = "开通"

    static let created = "已开通"

    static let joinTitle = "加入已有的素笺云服务"

    static let joinIntro = "在已经开通的另一台设备上，打开「我 → 素笺云服务 → 添加一台设备」，把那里显示的匹配码输进来。"

    static let joinField = "匹配码"

    static let joinButton = "加入"

    static let joinHint = "8 位字母数字，中间的短横线可以不输"

    static let waiting = "等另一台设备允许…"

    static let waitingHint = "另一台设备上会弹出一条申请，允许之后就加入好了。"

    static let denied = "另一台设备没有允许这次加入，可以让它再出一张码"

    static let joined = "已加入"

    static let cancel = "取消"

    static let retry = "重新输一次"

    static let missingCode = "匹配码还差几位"

    static let expired = "这次加入过期了，请重新输一张匹配码"

    /// 开通页与输码页上会出现的全部文字，测试用来查有没有不该出现的词。
    static let allTexts: [String] = [
        consent, createTitle, createIntro, createButton, created,
        joinTitle, joinIntro, joinField, joinButton, joinHint,
        waiting, waitingHint, denied, joined, cancel, retry, missingCode, expired,
    ]

    /// 给用户看的失败原因：这两种说法与桥给的一致，其余用桥的原文。
    static func failure(_ failure: BridgeFailure) -> String {
        switch failure.code {
        case .badCode: "匹配码不对或已经过期，请重新输一张"
        case .deviceLimit: "空间里的设备已经满了，先在旧设备上删一台再加"
        default: failure.message
        }
    }
}
```

- [ ] **Step 5：跑**

预期：两个文件的用例全过。

- [ ] **Step 6：提交**

```bash
git add cloud/ios/App/Space cloud/ios/Tests/SpaceFlowTests.swift cloud/ios/Tests/SpaceWordingTests.swift
git commit -m "feat(ios): 轮询的边界与这两页的文案做成可测的纯值"
```

---

## Task 6：`SpaceStore`

**Files:**
- Create: `cloud/ios/App/Space/SpaceStore.swift`

- [ ] **Step 1：实现**

```swift
// 开通与加入的状态：桥的调用都阻塞网络，放进后台任务；令牌不经过这里（桥直接写 cloud.toml）。
// 加入是两步：输码拿到 request_id 与 secret，再按 SpaceFlow 的节奏轮询到有结果。

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

    /// 输码之后的等待态；`nil` 表示没在等。
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

    private static let noGroup = "这个安装包没有开通 App Group，云服务用不了"

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
        let failure = await run { PairBridge.createSpace($0, device: UIDevice.current.name, crossBorderConsented: consented) }
        message = failure.map(SpaceWording.failure) ?? SpaceWording.created
        if failure == nil {
            finished = true
            refresh()
        }
    }

    /// 输码申请加入，成功后开始等对方允许。
    func join(code: String) async {
        guard MatchCode.isComplete(code) else {
            message = SpaceWording.missingCode
            return
        }
        waiting = true
        message = nil
        let device = UIDevice.current.name
        let result = await call { PairBridge.pairJoin($0, code: MatchCode.entered(code), device: device) }
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
            while !Task.isCancelled, flow.shouldPoll(now: SpaceStore.now) {
                try? await Task.sleep(for: .seconds(flow.nextDelay(now: SpaceStore.now)))
                if Task.isCancelled { return }
                let result = await self?.pollOnce(ticket) ?? .cancelled
                switch result {
                case .approved, .cancelled:
                    return
                case .pending, .denied:
                    continue
                }
            }
            // 等到过期还没结果：回到输码那一步，让用户重新来
            await self?.giveUpWaiting()
        }
    }

    private enum PollOutcome { case pending, denied, approved, cancelled }

    private func pollOnce(_ ticket: PairReply.Ticket) async -> PollOutcome {
        guard let file = fileProvider() else { return .cancelled }
        let result = await Task.detached(priority: .userInitiated) {
            PairBridge.pairPoll(file, requestId: ticket.requestId, secret: ticket.secret)
        }.value
        switch result {
        case .failure:
            // 单次失败不打断等待：网络抖一下不该让用户重输码
            return .pending
        case .success(let poll):
            switch poll.state {
            case .pending: return .pending
            case .denied: return .denied
            case .approved: return .approved
            }
        }
    }

    private func giveUpWaiting() {
        waiting = false
        polling = nil
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
    private func call<T>(_ work: @escaping @Sendable (URL) -> Result<T, BridgeFailure>) async -> Result<T, BridgeFailure> {
        guard let file = fileProvider() else {
            return .failure(BridgeFailure(code: .other, message: Self.noGroup))
        }
        busy = true
        defer { busy = false }
        return await Task.detached(priority: .userInitiated) { work(file) }.value
    }
}
```

- [ ] **Step 2：加状态机的测试**

轮询循环本身要连网才算，不单测；「什么时候停」与「等到过期怎么说」的判定留在 `SpaceFlow`（Task 5）与
`SpaceWording.expired` 里测，那两处已经覆盖。这一步只加 `SpaceStore` 自己的本地判定：

`Tests/SpaceStoreTests.swift`：

```swift
// 输码前的本地判定：不够 8 位就不发请求。

import XCTest
@testable import QingjianCloud

@MainActor
final class SpaceStoreTests: XCTestCase {
    func testAnIncompleteCodeIsRejectedBeforeAnyNetworkCall() async {
        let store = SpaceStore()
        var calls = 0
        store.fileProvider = {
            calls += 1
            return nil
        }
        await store.join(code: "K7P2-9")
        XCTAssertEqual(store.message, SpaceWording.missingCode)
        XCTAssertFalse(store.waiting)
        XCTAssertEqual(calls, 0, "不够 8 位不该碰桥")
    }
}
```

- [ ] **Step 3：跑**

```bash
cd cloud/ios && xcodegen generate && xcodebuild -project QingjianCloud.xcodeproj -scheme QingjianCloud \
  -destination 'platform=iOS Simulator,name=iPhone 17' -only-testing:QingjianCloudTests/SpaceStoreTests test
```

预期：全过。

- [ ] **Step 4：提交**

```bash
git add cloud/ios/App/Space cloud/ios/Tests/SpaceStoreTests.swift
git commit -m "feat(ios): 开通与加入的状态，加入按节奏轮询到有结果"
```

---

## Task 7：两页界面

**Files:**
- Create: `cloud/ios/App/Space/CreateSpaceView.swift`
- Create: `cloud/ios/App/Space/JoinSpaceView.swift`

**版式照系统表单（D6 未答的备选），交 UI 审计员。**

- [ ] **Step 1：开通页**

```swift
// 开通素笺云服务（05 的 2e 点「了解云服务」之后）：一句说明 + 出境同意 + 一个按钮。
// 设计稿没有这一屏（D6），先照系统表单做，交 UI 审计员过。

import SwiftUI

struct CreateSpaceView: View {
    @Bindable var store: SpaceStore

    @Environment(\.dismiss) private var dismiss

    var body: some View {
        Form {
            Section {
                Text(SpaceWording.createIntro)
            }
            Section {
                Toggle(isOn: $store.consented) {
                    Text(SpaceWording.consent).font(AppFont.footnote)
                }
                .toggleStyle(CheckboxToggleStyle())
                Link("了解更多", destination: PrivacyLinks.dataLocation)
                    .font(AppFont.footnote)
            }
            Section {
                Button(SpaceWording.createButton) { Task { await store.create() } }
                    .disabled(!store.consented || store.busy)
            } footer: {
                if let message = store.message { Text(message) }
            }
        }
        .navigationTitle(SpaceWording.createTitle)
        .navigationBarTitleDisplayMode(.inline)
        .disabled(store.busy)
        .onChange(of: store.finished) { _, done in if done { dismiss() } }
    }
}
```

- [ ] **Step 2：输码页**

```swift
// 在新设备上加入已有的空间：输一张 8 位匹配码 → 等旧设备允许 → 加入好了。
// 找回方式（Apple / 邮箱 / 微信）按 D7 往后放，这一页只有输码。设计稿没画（D6），照系统表单做。

import SwiftUI

struct JoinSpaceView: View {
    @Bindable var store: SpaceStore

    @Environment(\.dismiss) private var dismiss

    @State private var code = ""

    @FocusState private var focused: Bool

    var body: some View {
        Form {
            Section {
                Text(SpaceWording.joinIntro).font(AppFont.footnote)
            }
            if store.waiting {
                Section {
                    HStack(spacing: 10) {
                        ProgressView()
                        Text(SpaceWording.waiting)
                    }
                    Text(SpaceWording.waitingHint).font(AppFont.footnote).foregroundStyle(.secondary)
                    Button(SpaceWording.cancel) { store.cancelWaiting() }
                }
            } else {
                Section {
                    TextField(SpaceWording.joinField, text: $code)
                        .textInputAutocapitalization(.characters)
                        .autocorrectionDisabled()
                        .focused($focused)
                    Button(SpaceWording.joinButton) { Task { await store.join(code: code) } }
                        .disabled(!MatchCode.isComplete(code) || store.busy)
                } footer: {
                    Text(SpaceWording.joinHint).font(AppFont.footnote)
                }
            }
            if let message = store.message {
                Section { Text(message).foregroundStyle(.secondary) }
            }
        }
        .navigationTitle(SpaceWording.joinTitle)
        .navigationBarTitleDisplayMode(.inline)
        .onChange(of: store.finished) { _, done in if done { dismiss() } }
        .onAppear { focused = true }
    }
}
```

- [ ] **Step 3：跑（编译 + 全量单测）**

```bash
cd cloud/ios && xcodegen generate && xcodebuild -project QingjianCloud.xcodeproj -scheme QingjianCloud \
  -destination 'platform=iOS Simulator,name=iPhone 17' -only-testing:QingjianCloudTests test
```

预期：全过（这一件不改既有用例）。

- [ ] **Step 4：提交**

```bash
git add cloud/ios/App/Space
git commit -m "feat(ios): 开通页与输码页（照系统表单，等设计稿）"
```

---

## Task 8：接进「我」页与首次引导

**Files:**
- Modify: `cloud/ios/App/Me/MeView.swift`
- Modify: `cloud/ios/App/Onboarding/OnboardingView.swift`
- Modify: `cloud/ios/Tests/OnboardingTests.swift`

- [ ] **Step 1：`MeView` 加一行**

`Section("设置")` 之前插：

```swift
                    Section {
                        NavigationLink {
                            SpaceEntryView(store: space)
                        } label: {
                            LabeledContent("素笺云服务", value: space.signedIn == true ? "已开通" : "没开通")
                        }
                    }
```

`@State private var space = SpaceStore()`，`.task { space.refresh() }` 挂在最外层 `VStack` 上。
`Self.showsAccountEntry` 与 `AccountView(store: account)` 那条分支**删掉**（`AccountStore` 只留着给 `AccountView`，
本轮不再有入口——App 里就不再出现「账号」两个字）。

新建 `cloud/ios/App/Space/SpaceEntryView.swift`：

```swift
// 「我 → 素笺云服务」：没开通就开通，开通了先显示一句状态（设备页与出码是 1b Task 4）。

import SwiftUI

struct SpaceEntryView: View {
    @Bindable var store: SpaceStore

    var body: some View {
        Group {
            if store.signedIn == true {
                List {
                    Section {
                        Text("这台设备已经开通了素笺云服务。")
                    } footer: {
                        Text("设备列表、加一台设备、出匹配码在下一步做。").font(AppFont.footnote)
                    }
                }
            } else {
                CreateSpaceView(store: store)
            }
        }
        .onAppear { store.refresh() }
    }
}
```

- [ ] **Step 2：引导里的「了解云服务」改推开通页**

`OnboardingView.swift` 的 `showsCloudIntro` 改名 `showsOpenSpace`，`navigationDestination` 换成：

```swift
                .navigationDestination(isPresented: $showsOpenSpace) {
                    CreateSpaceView(store: space)
                }
```

`@State private var space = SpaceStore()`。`PlanStep(onLearnCloud: { showsOpenSpace = true })` 不改（名字不变）。
`CloudIntroView` 仍然给别处用（`MaterialsSection` 的「开通素笺云后，每天帮你整理成记忆卡」那一行），**不删**。

- [ ] **Step 3：`OnboardingTests` 补一条**

```swift
    func testTheStepTextsDoNotMentionAccounts() {
        for text in PlanStep.allTexts {
            for banned in ["账号", "登录", "注册"] {
                XCTAssertFalse(text.contains(banned), "「\(text)」里有「\(banned)」")
            }
        }
    }
```

- [ ] **Step 4：跑**

```bash
cd cloud/ios && xcodegen generate && xcodebuild -project QingjianCloud.xcodeproj -scheme QingjianCloud \
  -destination 'platform=iOS Simulator,name=iPhone 17' -only-testing:QingjianCloudTests test
```

预期：全过。

- [ ] **Step 5：提交**

```bash
git add cloud/ios/App cloud/ios/Tests
git commit -m "feat(ios): 我页与引导接上开通流程，App 里不再出现账号"
```

---

## Task 9：截图走查与文档

- [ ] **Step 1：模拟器走查（浅色 + 深色各一套）**

按 `cloud/ios/UITests/README.md` 起专用模拟器与种子数据，跑 `VisualWalkthrough` / `HomeFlow`，
再手工走到这四屏各截一张：我页（没开通）→ 开通页（未勾 / 已勾）→ 开通成功 → 输码页（输一半 / 等待中 / 失败）。
截图放 PR 描述，不进仓库。

- [ ] **Step 2：交 UI 审计员**

按 UI 清单「验收」那一节的方式交；重点说明两页是 D6 未定稿时照系统表单做的。

- [ ] **Step 3：文档**

- `cloud/docs/plans/2026-10-05-ui-implementation.md`：05 的 2e 一行改成「点「了解云服务」进开通页（原来进说明页）」；
  2j 一行去掉「账号」措辞与登录页；「待设计确认」的 D6 补一句「设备页 / 匹配码页本轮照系统表单做，出码页随 Task 4」。
- `cloud/docs/design.md`：账号那一节改成「不要账号」的说法（1b Task 6 的一部分）。
- 本文件末尾的「执行记录」补实际做法与偏差。

- [ ] **Step 4：提交**

```bash
git add cloud/docs
git commit -m "docs(cloud): 开通云服务的流程与截图走查"
```

---

## 验证

1. `cargo test --manifest-path cloud/Cargo.toml`、`cargo clippy --manifest-path cloud/Cargo.toml --all-targets -- -D warnings`。
2. `cd cloud/ios && xcodegen generate && xcodebuild -project QingjianCloud.xcodeproj -scheme QingjianCloud -destination 'platform=iOS Simulator,name=iPhone 17' -only-testing:QingjianCloudTests test`：既有用例 + 新增的全过。
3. 对着假服务端（或本机起的服务端）走通：开通 → `cloud.toml` 里出现 `sjt_` 令牌、我页显示「已开通」；
   另一台设备输码 → 等待 → 允许 → 两台都进了同一个空间。
4. 界面里 `grep -rn "账号\|登录\|注册" cloud/ios/App` 只剩 `App/Account/` 这一块（本轮不删代码，只撤入口）。

## 自查

- **上游覆盖**：上游 Task 3 的五件事——2e 入口（Task 8）、出境同意（Task 7 开通页）、建空间（Task 6/7）、
  新设备输码（Task 6/7）、去掉账号入口（Task 8）——都有对应 Task。找回、微信、出码页、2f 按文首的范围表切掉。
- **不许动**：`scope/scoped_learner.rs`、键盘那套、`CloudIntroView`（还被素材那一节用着）。
- **要定的**：出境同意那句话的定稿、D6 的版式——都在文首「范围」一节写明。
- **提示**：`PairBridge` 只做「传字符串 + 解 JSON」，判定与文案全在 `MatchCode` / `SpaceFlow` / `SpaceWording` 里，
  那三个有单测；`SpaceStore` 的轮询循环要连网，靠 `SpaceFlow` 的边界测试兜。
- **留了个小尾巴**：`CreateSpaceView` 用的 `CheckboxToggleStyle` 在 `App/Account/` 下。等账号那一套整个退场时，
  把它挪到 `App/` 或 `Shared/`，本轮不动。

---

## 执行记录

2026-10-06 执行完，八个 Task 各一个提交（`9ddbbc5` → `2b147f3`，加桥那个 `d4f8a34`）。

- **测试**：iOS `QingjianCloudTests` 248 条全过（原来 227，新增 21）；cloud 侧 338 条全过，`clippy -D warnings` 与 `fmt --check` 干净。
- **偏差一**：`BridgeFailure` 得加 `Error` 才能当 `Result` 的失败类型；`AccountStore.reaction` 与
  `AccountDecodeTests` 里对 `Code` 的穷尽写法跟着补了两个新 case。
- **偏差二**：`CreateSpaceView` 底下多了一个「已经有素笺云服务了？用匹配码加入」的入口——
  计划里 `JoinSpaceView` 没有进水口，不给一个的话新设备走不到那一页。
- **偏差三**：计划里 `SpaceStore.noGroupMessage` 是个静态属性，Swift 6 下 `@MainActor` 类的静态属性
  在 `nonisolated` 处引用不了，改成 `nonisolated static let noGroup`。
- **偏差四**：`PairBridge` 与 `SpaceStore` 的泛型都要 `Sendable`（结果要跨 `Task.detached` 回来）。
- **截图走查做了**（`UITests/SpaceShots.swift`，iPhone 17 模拟器，浅色）：我页那一行、开通页（未勾 / 已勾）、
  输码页（空 / 输满）。走查当场抓到两件事：一是开通页失败的提示会漏到输码页（已修，`5310de5`），
  二是**测试不能点「开通」**——随包的 `cloud.toml` 指向线上，按下去真的会发一次建空间请求（已改成只断言按钮可点）。
  深色那一套与「交 UI 审计员」还没做。
- **没做**：出码页、设备页、找回方式、2f 同意页按文首的范围表切给别的任务；出境同意那句话仍是草案。
