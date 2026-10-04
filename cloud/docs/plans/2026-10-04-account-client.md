# 素笺子项目 1：客户端账号登录 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 青简 Cloud 的客户端（协议、客户端库、iOS 桥与 App、Mac 同步模块）从「每台设备手填一个 `qjc_` 设备令牌」改成「用 Apple ID 或邮箱登录素笺账号、拿 `sjt_` 会话令牌」，四项云功能（剪贴板、同步、输入日志、大模型）缺省全关、按服务器上的同意记录逐项打开。

**Architecture:** 协议类型加在 `qingjian-cloud-proto`（服务端也依赖），HTTP 方法加在 `qingjian-cloud-client` 的 `Client`（阻塞 ureq），各同步线程把 403 当作「这项功能没开」停下并上报。iOS 由 Rust 桥（`qingjian-cloud-bridge`）完成登录、改开关、退出、删账号，令牌只在桥与 App Group 的 `cloud.toml` 之间流转，Swift 只拿到 JSON 与给用户看的错误；Mac 在输入法进程里用 `ASWebAuthenticationSession` 打开服务端的网页登录页，PKCE（verifier / challenge）换一次性码，再换会话令牌写进 `QingjianCloud/config.toml`。换账号时清掉旧的同步进度（基线、游标、离线队列）。

**Tech Stack:** Rust 2024（serde、thiserror、ureq 3、toml / toml_edit、sha2、base64、getrandom、percent-encoding）、objc2 0.6 + objc2-authentication-services 0.3.2 + block2 0.6、SwiftUI（iOS 17，Swift 6，AuthenticationServices、CryptoKit）、xcodegen。

**前置与顺序（必读）：**

- 在青简仓库 **`sujian` 分支**（GitHub 上原 mainline 改名而来，跟踪 `origin/sujian`，起点与 `local` 同为 `b10e100`）的检出 `/Users/liyuqing/sproot/qingjian-mainline` 上**直接提交**（用户决定），不另开分支、不碰 `local` 分支与 `/Users/liyuqing/sproot/qingjian-local`。
- **给服务端计划执行者的提示（不属于本计划的任务）**：`synon-ime` 的 `Cargo.toml` 现在按路径依赖 `../qingjian-local/cloud/crates/*`，要改成 `../qingjian-mainline/cloud/crates/*`，由服务端计划执行者在开始前处理。
- **Task 1、Task 2 完成并提交后**，服务端计划 `/Users/liyuqing/sproot/synon-ime/docs/superpowers/plans/2026-10-04-account-server.md` 才能开始（它的测试用这两个 crate 的类型与 `Client` 方法）。Task 1、2 的类型名、路径、签名是与服务端约定好的契约，**一个字都不要改**。
- Task 3–5 的代码与单测不依赖服务端；**联调与真机验证要等服务端部署**到 `https://pinyin.synon.ai`（Apple 五项与 SMTP 配好，`SUJIAN_APPLE_IOS_CLIENT_ID = app.qingjian.cloud`）。
- Task 3 删掉了 `qj_cloud_config_read/write`，iOS 工程要到 Task 4 做完才能再编过：**Task 3、4 连着做**。
- 写计划时 `qingjian-mainline` 的工作区里有一处与本计划无关的未提交改动（`cloud/docs/handoff.md`）。每次提交**只 `git add` 本任务列出的路径**，不要 `git add -A`；开工前 `git status --short` 看一眼，若又多出别人的改动，同样不要带进提交。
- 约定：一个类型一个文件、`//!` 文件头、中文注释、`thiserror`、导入写精确路径、无装饰分隔注释；提交信息 `<类型>(cloud): <中文说明>`。
- 所有 cargo 命令在 `/Users/liyuqing/sproot/qingjian-mainline/cloud` 下跑（独立 workspace）。

---

## 审查补充（Task 2 代码审查发现，Task 3、5 的执行者必须一并处理）

1. **关掉「同步」后再打开，本机基线要重置。** 服务端关闭 `sync` 会删掉这个用户云端的学习数据与配置；本机的学习数据基线与配置版本号还认为「已经在服务端」，
   之后重新打开只会推增量，被删掉的部分补不回去，配置还会走 409 冲突。所以**每次 `put_consent(Feature::Sync, …)` 成功之后**（无论开还是关），
   壳都要清掉本机学习数据与配置的同步进度：先停掉 `DataSync`，再删 `state_dir` 里 `LearningSync` / `ConfigSync` 的状态文件（实现时读
   `learning/`、`config_sync/` 里它们 `open` 时读写的文件名，不要整目录删，剪贴板进度与输入日志进度不属于这两项）。
   iOS 在桥的 `set_consent` 里做（Task 3），Mac 在 `AccountEvent::Consents` 与 `Forbidden` 的处理里做（Task 5）。给这个清理函数写单测（临时目录里放假状态文件，调用后消失、别的文件还在）。
2. **剪贴板在服务器上关了之后不再积压明文。** Task 2 修订让 uploader 收到 403 时清空离线队列，`Status::Disabled` 期间 `copy()` 不入队。
   Task 3、5 里「403 时本机开关记成关并扔掉剪贴板离线队列」的那段只需沿用，不要再自己实现一遍清队列；Task 5 里 `forget_clipboard_queue_if_off` 若与它重复就删掉。
3. **Mac 菜单文案**：`Status::Unauthorized` 现在的文案「令牌无效：在服务器上重新登记设备」已过时，Task 5 重写菜单时改成「登录已失效，请重新登录」；
   `data_line` 要认 `DataStatus.unauthorized`（显示「登录已失效」）与 `DataStatus.disabled`（对应开关显示为关，所有项都停了就不要显示「刚刚同步」）。
4. **`DataSync` 的 `disabled` 只增不减**：本实例内不会自动恢复，重新打开必须重建 `DataSync`；登录换 token 同样要重建。Mac 与 iOS 的重建时机已在 Task 3、5 的设计里，实现时别漏。
5. **键盘换引擎要先释放旧的再建新的（Task 4 的 Swift）。** 现在 `KeyboardViewController` 里 `model.replaceEngine(Self.openEngine(...))` 的参数先求值：新 `Session` 先建好、新 `DataSync` 线程已经启动，
   之后赋值才触发旧 `Engine` 的 `deinit`，而 `DataSync` 的 Drop 只置 `stop`、不 join，旧线程还可能在一轮 HTTP 里，结束时把进度文件写回，把重置或新账号的状态覆盖掉。
   实现时改成先 `model.replaceEngine(nil)` 释放旧的，再 `openEngine`。账号页里每次登录、退出、开关、删账号之后，键盘下次弹出重开会话走的就是这条路径，务必走对。
   （不要给 `DataSync` 的 Drop 加阻塞式 join：键盘扩展在主线程释放引擎，卡几秒用户能感觉到。）
6. **没开「完全访问」时，学习数据与进度在键盘扩展自己的容器里**，桥只清 App Group 里的 `cloud/`。当时键盘没有网络、没有进度可清；之后用户开了完全访问，旧容器里的进度不会被清。边缘情况，Task 4 不处理，在 `cloud/ios/README.md`（Task 6）里写一句已知限制。
7. **`cloud.toml` 新增 `user_id` 字段**（Task 3 修订）：用来区分「同一账号重登」与「换账号」，只有换账号才清整个 `cloud/` 进度。Mac 的 `config.toml` 同理要存 `user_id`（Task 5）并按同样规则判断：同账号重登保留进度，换账号清进度；`SessionGrant.user_id` 就是依据。

---

## 文件结构

```
cloud/crates/qingjian-cloud-proto/
  src/lib.rs                         改：mod / pub use、TOKEN_PREFIX = "sjt_"、9 个路径常量
  src/platform.rs                    新：Platform
  src/device.rs                      新：Device
  src/apple_client.rs                新：AppleClient
  src/apple_sign_in.rs               新：AppleSignIn
  src/email_start.rs                 新：EmailStart
  src/email_verify.rs                新：EmailVerify
  src/handoff_exchange.rs            新：HandoffExchange
  src/handoff_grant.rs               新：HandoffGrant
  src/session_grant.rs               新：SessionGrant
  src/feature.rs                     新：Feature
  src/consents.rs                    新：Consents
  src/put_consent.rs                 新：PutConsent
  src/identity_info.rs               新：IdentityInfo
  src/session_info.rs                新：SessionInfo
  src/account.rs                     新：Account
  tests/account.rs                   新：serde 往返

cloud/crates/qingjian-cloud-client/
  src/error.rs                       改：Forbidden / RateLimited 与映射、单测
  src/client.rs → src/client/mod.rs  移：加 `mod account;`
  src/client/account.rs              新：anonymous / 登录 / 账号 / 开关 / 注销 / 退出 / 删账号
  src/clipboard_sync/status.rs       改：Status::Disabled
  src/clipboard_sync/shared.rs       改：set_error 认 Forbidden
  src/clipboard_sync/listener.rs     改：Forbidden 与 Unauthorized 同样停
  src/clipboard_sync/uploader.rs     改：同上
  src/data_sync/data_status.rs       改：disabled / unauthorized
  src/data_sync/jobs.rs              改：403 停掉对应的项
  src/data_sync/mod.rs               改：open_jobs、run
cloud/crates/qingjian-cloud-mac/src/menu/mod.rs   改（Task 2）：status_line 认 Status::Disabled

cloud/crates/qingjian-cloud-bridge/
  src/cloud_config/mod.rs → src/cloud_config.rs   移并重写：开关缺省关、登录写令牌 / 清令牌 / 开关回写
  src/cloud_config/status.rs         删
  src/cloud_config/switches.rs       删
  src/account/mod.rs                 新：登录、开关、注销、退出、删账号
  src/account/status.rs              新：AccountStatus（账号页 JSON）
  src/account/ffi.rs                 新：qj_account_* C 接口
  src/lib.rs                         改：mod account、re-export、删 qj_cloud_config_*、owned / path_arg 改 pub(crate)
  src/session/mod.rs                 改：输入日志只看 logs
  src/session/cloud.rs               改：DataSync 按 sync / logs 分开
  include/qingjian_bridge.h          改：账号接口
  tests/cloud_config.rs              重写
  tests/settings.rs                  改：删掉开关测试

cloud/ios/
  project.yml                        改：App 加 Sign in with Apple
  App/App.entitlements               改：同上
  App/QingjianApp.swift              不改
  App/SetupView.swift                改：「青简 Cloud」入口换成「账号」
  App/Settings/SettingsBridge.swift  改：删 readCloud / writeCloud，take / decode 去掉 private
  App/Settings/SettingsStore.swift   改：删 cloud 状态
  App/Settings/CloudSettings.swift   删
  App/Settings/CloudSettingsView.swift 删
  App/Account/AccountState.swift     新
  App/Account/Consents.swift         新
  App/Account/CloudFeature.swift     新
  App/Account/AccountIdentity.swift  新
  App/Account/AccountDevice.swift    新
  App/Account/Nonce.swift            新
  App/Account/AccountBridge.swift    新
  App/Account/AccountStore.swift     新
  App/Account/AccountView.swift      新
  App/Account/SignInSections.swift   新
  App/Account/DeviceRow.swift        新
  scripts/build-bridge.sh            改：只写 server，不拷 cloud.local.toml
  scripts/install-device.sh          改：检查包里没有令牌
  cloud.example.toml                 改：删令牌，改成 cloud.toml 的样例

cloud/crates/qingjian-cloud-mac/
  Cargo.toml                         改：依赖
  src/lib.rs                         改：mod account
  src/config.rs                      重写：token 由登录写入、四个开关、toml_edit 改写
  src/account/mod.rs                 新：verifier / challenge / 登录地址 / 回调解析 / 设备名 / 错误文案
  src/account/event.rs               新：AccountEvent
  src/account/anchor.rs              新：LoginAnchor（展示锚点，macOS）
  src/account/web_login.rs           新：WebLogin（ASWebAuthenticationSession，macOS）
  src/menu/mod.rs                    重写：账号行、开关、tag
  src/menu/account_menu.rs           新：AccountMenu
  src/menu/display.rs                改：SignedOut / SignedIn
  src/service.rs → src/service/mod.rs 移并改
  src/service/account.rs             新：impl Service 的账号部分
  README.md                          改：安装与配置
Cargo.lock（仓库根）                  改：输入法链进 cloud-mac 的新依赖
cloud/docs/fork-patch.md             改：Cargo.lock 那一行

cloud/docs/design.md                 改（Task 6）
cloud/ios/README.md                  改（Task 6）
cloud/README.md                      改（Task 6）
```

---

## Task 1: qingjian-cloud-proto 的账号类型与路径

**Files:**
- Create: `cloud/crates/qingjian-cloud-proto/src/{platform,device,apple_client,apple_sign_in,email_start,email_verify,handoff_exchange,handoff_grant,session_grant,feature,consents,put_consent,identity_info,session_info,account}.rs`
- Modify: `cloud/crates/qingjian-cloud-proto/src/lib.rs`
- Test: `cloud/crates/qingjian-cloud-proto/tests/account.rs`

- [ ] **Step 1: 写失败的测试**

新建 `cloud/crates/qingjian-cloud-proto/tests/account.rs`：

```rust
//! 账号相关的 JSON 形状：服务端（synon-ime）与各客户端按这里的字段名对接。

use qingjian_cloud_proto::{
    Account, AppleClient, AppleSignIn, Consents, Device, EmailVerify, Feature, HandoffExchange,
    HandoffGrant, IdentityInfo, Platform, PutConsent, SessionGrant, SessionInfo, TOKEN_PREFIX,
};
use serde_json::json;

fn iphone() -> Device {
    Device {
        name: "iPhone".to_owned(),
        platform: Platform::Ios,
    }
}

#[test]
fn feature_is_snake_case_and_parses_back() {
    assert_eq!(serde_json::to_value(Feature::InputLog).unwrap(), json!("input_log"));
    assert_eq!(serde_json::to_value(Feature::Clipboard).unwrap(), json!("clipboard"));
    for feature in Feature::ALL {
        assert_eq!(Feature::parse(feature.as_str()), Some(feature));
        assert_eq!(
            serde_json::to_value(feature).unwrap(),
            json!(feature.as_str())
        );
    }
    assert_eq!(Feature::parse("inputlog"), None);
}

#[test]
fn platform_and_client_are_lowercase() {
    assert_eq!(serde_json::to_value(Platform::Macos).unwrap(), json!("macos"));
    assert_eq!(serde_json::to_value(Platform::Web).unwrap(), json!("web"));
    assert_eq!(serde_json::to_value(AppleClient::Ios).unwrap(), json!("ios"));
    assert_eq!(
        serde_json::from_value::<Platform>(json!("ios")).unwrap(),
        Platform::Ios
    );
}

#[test]
fn apple_sign_in_without_challenge_omits_it() {
    let request = AppleSignIn {
        identity_token: "jwt".to_owned(),
        authorization_code: "code".to_owned(),
        nonce: "raw".to_owned(),
        client: AppleClient::Ios,
        device: iphone(),
        challenge: None,
    };
    let value = serde_json::to_value(&request).unwrap();
    assert_eq!(
        value,
        json!({
            "identity_token": "jwt", "authorization_code": "code", "nonce": "raw",
            "client": "ios", "device": { "name": "iPhone", "platform": "ios" }
        })
    );
    assert_eq!(serde_json::from_value::<AppleSignIn>(value).unwrap(), request);
}

#[test]
fn email_verify_with_challenge_round_trips() {
    let request = EmailVerify {
        email: "a@b.c".to_owned(),
        code: "123456".to_owned(),
        device: Device {
            name: "web".to_owned(),
            platform: Platform::Web,
        },
        challenge: Some("S256".to_owned()),
    };
    let value = serde_json::to_value(&request).unwrap();
    assert_eq!(value["challenge"], json!("S256"));
    assert_eq!(serde_json::from_value::<EmailVerify>(value).unwrap(), request);
    let without: EmailVerify = serde_json::from_value(json!({
        "email": "a@b.c", "code": "1", "device": { "name": "n", "platform": "macos" }
    }))
    .unwrap();
    assert_eq!(without.challenge, None);
}

#[test]
fn grants_and_handoff_round_trip() {
    let grant = SessionGrant {
        token: format!("{TOKEN_PREFIX}abc"),
        user_id: 42,
        session_id: 7,
        new_user: true,
    };
    let value = serde_json::to_value(&grant).unwrap();
    assert_eq!(
        value,
        json!({ "token": "sjt_abc", "user_id": 42, "session_id": 7, "new_user": true })
    );
    assert_eq!(serde_json::from_value::<SessionGrant>(value).unwrap(), grant);
    assert_eq!(
        serde_json::to_value(HandoffGrant { handoff: "h".to_owned() }).unwrap(),
        json!({ "handoff": "h" })
    );
    let exchange = HandoffExchange {
        handoff: "h".to_owned(),
        verifier: "v".to_owned(),
        device: iphone(),
    };
    let value = serde_json::to_value(&exchange).unwrap();
    assert_eq!(serde_json::from_value::<HandoffExchange>(value).unwrap(), exchange);
}

#[test]
fn consents_default_off_and_follow_feature() {
    let mut consents = Consents::default();
    for feature in Feature::ALL {
        assert!(!consents.get(feature));
    }
    consents.set(Feature::InputLog, true);
    assert!(consents.input_log && consents.get(Feature::InputLog));
    assert_eq!(
        serde_json::to_value(consents).unwrap(),
        json!({ "clipboard": false, "sync": false, "input_log": true, "llm": false })
    );
    assert_eq!(
        serde_json::to_value(PutConsent { enabled: true }).unwrap(),
        json!({ "enabled": true })
    );
}

#[test]
fn account_round_trips_with_null_fields() {
    let json = json!({
        "identities": [{ "provider": "apple", "label": null }, { "provider": "email", "label": "a@b.c" }],
        "sessions": [{ "id": 7, "name": "iPhone", "platform": "ios", "created_at": 1, "last_seen": null, "current": true }],
        "consents": { "clipboard": true, "sync": false, "input_log": false, "llm": true }
    });
    let account: Account = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(
        account.identities[0],
        IdentityInfo {
            provider: "apple".to_owned(),
            label: None
        }
    );
    assert_eq!(
        account.sessions[0],
        SessionInfo {
            id: 7,
            name: "iPhone".to_owned(),
            platform: Platform::Ios,
            created_at: 1,
            last_seen: None,
            current: true
        }
    );
    assert!(account.consents.clipboard && account.consents.llm);
    assert_eq!(serde_json::to_value(&account).unwrap(), json);
}

#[test]
fn token_prefix_is_sujian() {
    assert_eq!(TOKEN_PREFIX, "sjt_");
}
```

- [ ] **Step 2: 跑测试，确认失败**

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud && cargo test -p qingjian-cloud-proto --test account`
Expected: 编译失败，`error[E0432]: unresolved imports qingjian_cloud_proto::Account, …`。

- [ ] **Step 3: 实现类型（一个类型一个文件）**

`src/platform.rs`：

```rust
//! 设备的平台：建会话时报给服务端，设备列表里显示。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Platform {
    Ios,
    Macos,
    /// 服务端的网页登录页。
    Web,
}
```

`src/device.rs`：

```rust
//! 登录请求里带的设备：服务端建会话时记下，设备列表里显示、可注销。

use serde::{Deserialize, Serialize};

use crate::Platform;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Device {
    /// 设备名（iPhone 的名字、Mac 的电脑名）。
    pub name: String,

    pub platform: Platform,
}
```

`src/apple_client.rs`：

```rust
//! Apple 登录的发起方：决定服务端用哪个 client_id 核对 `identity_token` 的 `aud`。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppleClient {
    /// iOS App：client_id 是 App 的 bundle id。
    Ios,

    /// 网页登录页：client_id 是 Services ID。
    Web,
}
```

`src/apple_sign_in.rs`：

```rust
//! `POST /v1/auth/apple` 的请求体。

use serde::{Deserialize, Serialize};

use crate::{AppleClient, Device};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppleSignIn {
    /// Apple 签的 JWT。
    pub identity_token: String,

    /// 换 refresh_token 用的一次性授权码。
    pub authorization_code: String,

    /// 原始 nonce；交给 Apple 的是它的 SHA-256 十六进制。
    pub nonce: String,

    pub client: AppleClient,

    pub device: Device,

    /// 只有网页登录带：S256(verifier)，有它时服务端返回 HandoffGrant 而不是 SessionGrant。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub challenge: Option<String>,
}
```

`src/email_start.rs`：

```rust
//! `POST /v1/auth/email/start` 的请求体：给这个邮箱发 6 位验证码，成功是 204。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailStart {
    pub email: String,
}
```

`src/email_verify.rs`：

```rust
//! `POST /v1/auth/email/verify` 的请求体：邮箱加验证码换会话（首次验证即注册）。

use serde::{Deserialize, Serialize};

use crate::Device;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmailVerify {
    pub email: String,

    /// 邮件里的 6 位数字。
    pub code: String,

    pub device: Device,

    /// 只有网页登录带：S256(verifier)，有它时服务端返回 HandoffGrant。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub challenge: Option<String>,
}
```

`src/handoff_exchange.rs`：

```rust
//! `POST /v1/auth/handoff` 的请求体：Mac 拿网页登录回跳的一次性码与本进程里的 verifier 换会话。

use serde::{Deserialize, Serialize};

use crate::Device;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoffExchange {
    /// 回跳地址 `sujian://auth?handoff=…` 里的一次性码。
    pub handoff: String,

    /// PKCE 的 verifier，服务端核对 S256(verifier) 等于登录时的 challenge。
    pub verifier: String,

    pub device: Device,
}
```

`src/handoff_grant.rs`：

```rust
//! 带 `challenge` 的登录（网页登录页）的响应：60 秒有效的一次性码，页面拿它回跳给 Mac。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoffGrant {
    pub handoff: String,
}
```

`src/session_grant.rs`：

```rust
//! 登录成功的响应：会话令牌与账号、会话的 id。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionGrant {
    /// `sjt_` 开头的会话令牌，之后放在 `Authorization: Bearer …` 里。
    pub token: String,

    pub user_id: i64,

    pub session_id: i64,

    /// 这次登录新建了账号。
    pub new_user: bool,
}
```

`src/feature.rs`：

```rust
//! 要用户单独同意才开的云功能；服务端没开的功能，对应接口返回 403。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Feature {
    /// 跨设备剪贴板。
    Clipboard,

    /// 学习数据与 `config.toml` 同步。
    Sync,

    /// 上传输入日志。
    InputLog,

    /// 大模型代理（云联想、润色）。
    Llm,
}

impl Feature {
    pub const ALL: [Feature; 4] = [
        Feature::Clipboard,
        Feature::Sync,
        Feature::InputLog,
        Feature::Llm,
    ];

    /// 路径与 JSON 里的名字（`PUT /v1/consents/{feature}`）。
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Clipboard => "clipboard",
            Self::Sync => "sync",
            Self::InputLog => "input_log",
            Self::Llm => "llm",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|f| f.as_str() == text)
    }
}
```

`src/consents.rs`：

```rust
//! 四项功能各自开没开（同意记录）；新用户全关。

use serde::{Deserialize, Serialize};

use crate::Feature;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Consents {
    pub clipboard: bool,

    pub sync: bool,

    pub input_log: bool,

    pub llm: bool,
}

impl Consents {
    pub fn get(&self, feature: Feature) -> bool {
        match feature {
            Feature::Clipboard => self.clipboard,
            Feature::Sync => self.sync,
            Feature::InputLog => self.input_log,
            Feature::Llm => self.llm,
        }
    }

    pub fn set(&mut self, feature: Feature, enabled: bool) {
        match feature {
            Feature::Clipboard => self.clipboard = enabled,
            Feature::Sync => self.sync = enabled,
            Feature::InputLog => self.input_log = enabled,
            Feature::Llm => self.llm = enabled,
        }
    }
}
```

`src/put_consent.rs`：

```rust
//! `PUT /v1/consents/{feature}` 的请求体；响应是新的 [`crate::Consents`]。关掉即删云端这部分数据。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct PutConsent {
    pub enabled: bool,
}
```

`src/identity_info.rs`：

```rust
//! 账号的一种登录方式。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentityInfo {
    /// `apple` / `email`（以后 `google` / `x`）。
    pub provider: String,

    /// 给用户看的标识（邮箱地址；Apple 没给邮箱时为空）。
    pub label: Option<String>,
}
```

`src/session_info.rs`：

```rust
//! 账号下的一台已登录设备（会话）。

use serde::{Deserialize, Serialize};

use crate::Platform;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: i64,

    pub name: String,

    pub platform: Platform,

    /// Unix 毫秒。
    pub created_at: i64,

    /// 最近一次请求的时间，Unix 毫秒；登录后还没请求过为空。
    pub last_seen: Option<i64>,

    /// 是不是发这个请求的设备。
    pub current: bool,
}
```

`src/account.rs`：

```rust
//! `GET /v1/account` 的响应：登录方式、设备与功能开关。

use serde::{Deserialize, Serialize};

use crate::{Consents, IdentityInfo, SessionInfo};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub identities: Vec<IdentityInfo>,

    pub sessions: Vec<SessionInfo>,

    pub consents: Consents,
}
```

- [ ] **Step 4: 改 `src/lib.rs`**

把 `mod` 列表与 `pub use` 列表（第 5–25 行）整段换成：

```rust
mod account;
mod apple_client;
mod apple_sign_in;
mod config_doc;
mod consents;
mod device;
mod email_start;
mod email_verify;
mod event;
mod event_kind;
mod event_page;
mod feature;
mod handoff_exchange;
mod handoff_grant;
mod identity_info;
mod input_log;
mod learning_page;
mod learning_push;
mod learning_row;
mod platform;
mod push_clip;
mod put_consent;
mod session_grant;
mod session_info;
mod whoami;

pub use account::Account;
pub use apple_client::AppleClient;
pub use apple_sign_in::AppleSignIn;
pub use config_doc::{ConfigDoc, PutConfig};
pub use consents::Consents;
pub use device::Device;
pub use email_start::EmailStart;
pub use email_verify::EmailVerify;
pub use event::Event;
pub use event_kind::EventKind;
pub use event_page::EventPage;
pub use feature::Feature;
pub use handoff_exchange::HandoffExchange;
pub use handoff_grant::HandoffGrant;
pub use identity_info::IdentityInfo;
pub use input_log::{InputLogLine, InputLogPage, InputLogPush};
pub use learning_page::LearningPage;
pub use learning_push::{CountDelta, LearningPush, MAX_LEARNING_PUSH, SetDelete, SetPut};
pub use learning_row::LearningRow;
pub use platform::Platform;
pub use push_clip::PushClip;
pub use put_consent::PutConsent;
pub use session_grant::SessionGrant;
pub use session_info::SessionInfo;
pub use whoami::Whoami;
```

把 `TOKEN_PREFIX` 那两行（第 33–34 行）换成：

```rust
/// 会话令牌的前缀，便于在配置里认出来；旧的设备令牌是 `qjc_`，已作废。
pub const TOKEN_PREFIX: &str = "sjt_";
```

在 `PATH_HEALTH` 之后、`PATH_WHOAMI` 之前插入：

```rust
/// `POST` Apple 登录（不要鉴权）。
pub const PATH_AUTH_APPLE: &str = "/v1/auth/apple";

/// `POST` 给邮箱发验证码（不要鉴权），成功是 204。
pub const PATH_AUTH_EMAIL_START: &str = "/v1/auth/email/start";

/// `POST` 邮箱加验证码登录（不要鉴权）。
pub const PATH_AUTH_EMAIL_VERIFY: &str = "/v1/auth/email/verify";

/// `POST` 网页登录的一次性码加 verifier 换会话（不要鉴权）。
pub const PATH_AUTH_HANDOFF: &str = "/v1/auth/handoff";

/// `GET` 账号；`DELETE` 删账号。
pub const PATH_ACCOUNT: &str = "/v1/account";

/// `DELETE /v1/sessions/{id}` 注销某台设备，`DELETE /v1/sessions/current` 退出登录。
pub const PATH_SESSIONS: &str = "/v1/sessions";

/// `PUT /v1/consents/{feature}` 开关一项功能。
pub const PATH_CONSENTS: &str = "/v1/consents";

/// 网页登录页（Mac 用 `ASWebAuthenticationSession` 打开）。
pub const PATH_LOGIN: &str = "/login";

/// 网页登录回跳的 URL scheme：`sujian://auth?handoff=…`。
pub const LOGIN_CALLBACK_SCHEME: &str = "sujian";
```

`PATH_WHOAMI` 的注释 `查询当前令牌对应的设备。` 改成 `查询当前会话对应的设备名与本用户最新的 seq。`。

- [ ] **Step 5: 跑测试，确认通过**

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud && cargo test -p qingjian-cloud-proto && cargo clippy -p qingjian-cloud-proto --all-targets -- -D warnings`
Expected: `tests/account.rs` 8 个测试与原有 `tests/json.rs` 全部 PASS，clippy 无警告。

- [ ] **Step 6: 提交**

```bash
cd /Users/liyuqing/sproot/qingjian-mainline
git add cloud/crates/qingjian-cloud-proto
git commit -m "$(cat <<'EOF'
feat(cloud): 协议加素笺账号的登录、会话与功能开关类型

TOKEN_PREFIX 改成 sjt_，旧的 qjc_ 设备令牌作废；路径常量与类型是和 synon-ime 服务端约定的契约。

EOF
)"
```

---

## Task 2: qingjian-cloud-client 的账号接口与 403 / 429

**Files:**
- Modify: `cloud/crates/qingjian-cloud-client/src/error.rs`
- Move: `cloud/crates/qingjian-cloud-client/src/client.rs` → `src/client/mod.rs`
- Create: `cloud/crates/qingjian-cloud-client/src/client/account.rs`
- Modify: `src/clipboard_sync/{status,shared,listener,uploader}.rs`、`src/data_sync/{data_status,jobs,mod}.rs`
- Modify: `cloud/crates/qingjian-cloud-mac/src/menu/mod.rs`（`status_line` 的 match 要认新变体，否则 workspace 编不过）
- Test: `src/error.rs` 内的 `#[cfg(test)] mod tests`

现状（写计划时读到的）：`supervise.rs` 只管线程体 panic 与 `Exit::Retry`，**不看 `ClientError`**。`Unauthorized` 的处理在三处：
剪贴板监听线程 `listener.rs` 与上传线程 `uploader.rs` 遇到它把状态设成 `Status::Unauthorized`、固定睡 `UNAUTHORIZED_RETRY`（300 秒）再试；
学习 / 配置 / 输入日志的 `data_sync` 不区分错误种类，一律按 1 秒起翻倍、封顶 300 秒的退避重试，只把错误文字放进 `DataStatus.error`。
另外现在 403 落进 `Rejected`，上传线程会把这条剪贴板**丢掉**。本任务让 403 与 401 一样停下、上报、不按退避空转：剪贴板两线程并进 `Unauthorized` 的分支；`data_sync` 把收到 403 的那一项从本轮起停掉（不再请求）并记进 `DataStatus.disabled`，收到 401 记 `unauthorized` 并按最长间隔等。

- [ ] **Step 1: 写失败的测试**

在 `cloud/crates/qingjian-cloud-client/src/error.rs` 末尾追加：

```rust
#[cfg(test)]
mod tests {
    use super::ClientError;

    #[test]
    fn status_codes_map_to_variants() {
        assert!(matches!(
            ClientError::from(ureq::Error::StatusCode(401)),
            ClientError::Unauthorized
        ));
        assert!(matches!(
            ClientError::from(ureq::Error::StatusCode(403)),
            ClientError::Forbidden(message) if message.is_empty()
        ));
        assert!(matches!(
            ClientError::from(ureq::Error::StatusCode(429)),
            ClientError::RateLimited
        ));
        assert!(matches!(
            ClientError::from(ureq::Error::StatusCode(409)),
            ClientError::Rejected { status: 409, .. }
        ));
        assert!(matches!(
            ClientError::from(ureq::Error::StatusCode(503)),
            ClientError::Unreachable(_)
        ));
    }

    #[test]
    fn forbidden_and_rate_limited_are_not_retryable() {
        assert!(!ClientError::Forbidden(String::new()).is_retryable());
        assert!(!ClientError::RateLimited.is_retryable());
        assert!(!ClientError::Unauthorized.is_retryable());
        assert!(ClientError::Unreachable("x".to_owned()).is_retryable());
    }
}
```

- [ ] **Step 2: 跑测试，确认失败**

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud && cargo test -p qingjian-cloud-client error::tests`
Expected: 编译失败，`error[E0599]: no variant or associated item named Forbidden found for enum ClientError`（`RateLimited` 同）。

- [ ] **Step 3: 实现 `error.rs`**

整个文件替换为（测试模块保留在末尾，即 Step 1 追加的那段）：

```rust
//! 客户端错误。按处理方式分：连不上（重试）、令牌无效或功能没开（停下等用户登录或打开）、请求被拒（丢掉这一条）。

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    /// 网络不通、超时、TLS 失败、服务端 5xx：过一会儿重试。
    #[error("server unreachable: {0}")]
    Unreachable(String),

    /// 令牌无效：退出登录了、设备被注销或账号已删（401）。
    #[error("session token rejected")]
    Unauthorized,

    /// 这项功能在服务器上没开，或访问了别人的东西（403）：停下，等用户在设置里打开，不重试。
    #[error("forbidden: {0}")]
    Forbidden(String),

    /// 请求太频繁（429，验证码发得太勤、大模型超了每日上限）：这次放弃，交给用户稍后再试。
    #[error("rate limited")]
    RateLimited,

    /// 服务端拒绝了这个请求（其余 4xx），重试也没用。
    #[error("request rejected ({status}): {message}")]
    Rejected { status: u16, message: String },

    /// 响应不是预期的 JSON。
    #[error("bad response: {0}")]
    BadResponse(String),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

impl ClientError {
    /// 稍后重试有没有意义。
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Unreachable(_) | Self::Io(_))
    }
}

impl From<ureq::Error> for ClientError {
    fn from(error: ureq::Error) -> Self {
        match error {
            ureq::Error::StatusCode(401) => Self::Unauthorized,
            // ureq 开了 http_status_as_error，响应体拿不到，原因留空
            ureq::Error::StatusCode(403) => Self::Forbidden(String::new()),
            ureq::Error::StatusCode(429) => Self::RateLimited,
            ureq::Error::StatusCode(status) if (400..500).contains(&status) => Self::Rejected {
                status,
                message: String::new(),
            },
            other => Self::Unreachable(other.to_string()),
        }
    }
}
```

- [ ] **Step 4: 跑测试，确认通过**

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud && cargo test -p qingjian-cloud-client error::tests`
Expected: 2 个测试 PASS。

- [ ] **Step 5: `Client` 拆目录，加账号方法**

```bash
cd /Users/liyuqing/sproot/qingjian-mainline/cloud/crates/qingjian-cloud-client/src
mkdir client && git mv client.rs client/mod.rs
```

在 `client/mod.rs` 的文件头 `//! 阻塞式 HTTP 客户端，一个方法对应一个接口。` 改成：

```rust
//! 阻塞式 HTTP 客户端，一个方法对应一个接口。账号相关的（登录、开关、注销、删账号）在 `account.rs`。
```

并在它下面空一行后插入：

```rust
mod account;
```

（`account.rs` 是子模块，直接用父模块的私有字段 `agent`、私有方法 `url` / `bearer` 与自由函数 `json`，不用改可见性。）

新建 `client/account.rs`：

```rust
//! 账号相关的接口：登录（不带令牌）、账号信息、功能开关、注销设备、退出登录、删账号。

use qingjian_cloud_proto::{
    Account, AppleSignIn, Consents, EmailStart, EmailVerify, Feature, HandoffExchange,
    PATH_ACCOUNT, PATH_AUTH_APPLE, PATH_AUTH_EMAIL_START, PATH_AUTH_EMAIL_VERIFY,
    PATH_AUTH_HANDOFF, PATH_CONSENTS, PATH_SESSIONS, PutConsent, SessionGrant,
};

use super::{Client, json};
use crate::ClientError;

impl Client {
    /// 登录接口不要令牌。
    pub fn anonymous(server: &str) -> Self {
        Self::new(server, "")
    }

    /// Apple 登录，不带 `Authorization`。带了 `challenge` 的请求服务端回的是一次性码，不走这个方法。
    pub fn sign_in_apple(&self, request: &AppleSignIn) -> Result<SessionGrant, ClientError> {
        let mut response = self
            .agent
            .post(self.url(PATH_AUTH_APPLE))
            .send_json(request)?;
        json(response.body_mut().read_json())
    }

    /// 给邮箱发 6 位验证码；成功是 204，没有响应体。
    pub fn email_start(&self, email: &str) -> Result<(), ClientError> {
        let request = EmailStart {
            email: email.to_owned(),
        };
        self.agent
            .post(self.url(PATH_AUTH_EMAIL_START))
            .send_json(&request)?;
        Ok(())
    }

    pub fn email_verify(&self, request: &EmailVerify) -> Result<SessionGrant, ClientError> {
        let mut response = self
            .agent
            .post(self.url(PATH_AUTH_EMAIL_VERIFY))
            .send_json(request)?;
        json(response.body_mut().read_json())
    }

    /// 网页登录回跳的一次性码加 verifier 换会话。
    pub fn exchange_handoff(&self, request: &HandoffExchange) -> Result<SessionGrant, ClientError> {
        let mut response = self
            .agent
            .post(self.url(PATH_AUTH_HANDOFF))
            .send_json(request)?;
        json(response.body_mut().read_json())
    }

    pub fn account(&self) -> Result<Account, ClientError> {
        let mut response = self
            .agent
            .get(self.url(PATH_ACCOUNT))
            .header("Authorization", self.bearer())
            .call()?;
        json(response.body_mut().read_json())
    }

    /// 开关一项功能，返回服务器上新的四项开关。关掉时服务器删掉这部分云端数据。
    pub fn put_consent(&self, feature: Feature, enabled: bool) -> Result<Consents, ClientError> {
        let mut response = self
            .agent
            .put(self.url(&format!("{PATH_CONSENTS}/{}", feature.as_str())))
            .header("Authorization", self.bearer())
            .send_json(&PutConsent { enabled })?;
        json(response.body_mut().read_json())
    }

    /// 注销本账号的某台设备。
    pub fn revoke_session(&self, id: i64) -> Result<(), ClientError> {
        self.agent
            .delete(self.url(&format!("{PATH_SESSIONS}/{id}")))
            .header("Authorization", self.bearer())
            .call()?;
        Ok(())
    }

    /// 退出登录：注销发请求的这台设备。
    pub fn sign_out(&self) -> Result<(), ClientError> {
        self.agent
            .delete(self.url(&format!("{PATH_SESSIONS}/current")))
            .header("Authorization", self.bearer())
            .call()?;
        Ok(())
    }

    /// 删账号：服务器删掉这个账号的全部数据，所有设备的会话立即失效。
    pub fn delete_account(&self) -> Result<(), ClientError> {
        self.agent
            .delete(self.url(PATH_ACCOUNT))
            .header("Authorization", self.bearer())
            .call()?;
        Ok(())
    }
}
```

- [ ] **Step 6: 剪贴板线程认 403**

`src/clipboard_sync/status.rs` 整个文件替换为：

```rust
//! 连接状态，壳用来改菜单栏图标与提示。

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// 刚启动，还没连上过。
    Connecting,

    /// SSE 连着或刚刚请求成功。
    Online,

    /// 连不上，正在按退避重试；带最近一次的错误。
    Offline(String),

    /// 令牌被拒（401）：要用户重新登录。
    Unauthorized,

    /// 服务器上没开跨设备剪贴板（403）：停下，等用户在设置里打开。
    Disabled,
}
```

`src/clipboard_sync/shared.rs` 的 `set_error`（第 74–80 行）替换为：

```rust
    /// 按错误种类改状态。
    pub fn set_error(&self, error: &ClientError) {
        match error {
            ClientError::Unauthorized => self.set_status(Status::Unauthorized),
            ClientError::Forbidden(_) => self.set_status(Status::Disabled),
            other => self.set_status(Status::Offline(other.to_string())),
        }
    }
```

`src/clipboard_sync/listener.rs`：常量（第 10–11 行）替换为：

```rust
/// 令牌被拒或服务器上没开剪贴板后多久再试：这两种都要用户操作，不按退避空转。
const UNAUTHORIZED_RETRY: Duration = Duration::from_secs(300);
```

`run` 里的 match（第 42–45 行）替换为：

```rust
                let delay = match error {
                    ClientError::Unauthorized | ClientError::Forbidden(_) => UNAUTHORIZED_RETRY,
                    _ => backoff.next_delay(),
                };
```

`src/clipboard_sync/uploader.rs`：常量（第 13–14 行）替换为：

```rust
/// 令牌被拒或服务器上没开剪贴板后多久再试；队列留着，不丢。
const UNAUTHORIZED_RETRY: Duration = Duration::from_secs(300);
```

`run` 末尾 `Err(error) => { … }` 分支里的 match（第 64–67 行）替换为：

```rust
                delay = match error {
                    ClientError::Unauthorized | ClientError::Forbidden(_) => UNAUTHORIZED_RETRY,
                    _ => backoff.next_delay(),
                };
```

（`Forbidden` 现在不再落进上面的 `Rejected` 分支，所以 403 不会把这条剪贴板丢掉。）

- [ ] **Step 7: 学习 / 配置 / 输入日志线程认 403 与 401**

`src/data_sync/data_status.rs` 整个文件替换为：

```rust
//! 学习数据同步的状态，壳显示在菜单里。

use qingjian_cloud_proto::Feature;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DataStatus {
    /// 上次成功的时间，Unix 毫秒。
    pub last_ok_ms: Option<i64>,

    /// 收件箱在等输入法合并（输入法没在用，或者装的不是打了补丁的版本）。
    pub waiting_for_ime: bool,

    /// 上次失败的原因；成功后清掉。
    pub error: Option<String>,

    /// 出现过配置冲突（较旧的一份已存成备份）。
    pub config_conflict: bool,

    /// 服务器上没开、已经停掉的功能（403）；壳据此把对应开关显示为关。
    pub disabled: Vec<Feature>,

    /// 令牌被拒（401）：停在这里等重新登录。
    pub unauthorized: bool,
}
```

`src/data_sync/jobs.rs` 整个文件替换为：

```rust
//! 后台线程每轮要做的几项同步。某一项服务器说没开（403）就从这一轮起停掉它，别的照常。

use qingjian_cloud_proto::Feature;

use crate::config_sync::{ConfigOutcome, ConfigSync};
use crate::{ClientError, InputLogSync, LearningOutcome, LearningSync};

/// 开着的几项同步；关掉的为 `None`。
pub struct Jobs {
    pub learning: Option<LearningSync>,

    pub settings: Option<ConfigSync>,

    pub logs: Option<InputLogSync>,

    /// 服务器说没开（403）而停掉的功能。
    pub disabled: Vec<Feature>,
}

impl Jobs {
    pub fn cycle(&mut self) -> Result<(LearningOutcome, ConfigOutcome), ClientError> {
        let outcome = match self.learning.as_mut().map(LearningSync::cycle) {
            Some(Err(ClientError::Forbidden(_))) => {
                self.learning = None;
                self.disable(Feature::Sync);
                LearningOutcome::default()
            }
            Some(result) => result?,
            None => LearningOutcome::default(),
        };
        let config = match self.settings.as_mut().map(ConfigSync::cycle) {
            Some(Err(ClientError::Forbidden(_))) => {
                self.settings = None;
                self.disable(Feature::Sync);
                ConfigOutcome::Unchanged
            }
            Some(result) => result?,
            None => ConfigOutcome::Unchanged,
        };
        match self.logs.as_mut().map(InputLogSync::cycle) {
            Some(Err(ClientError::Forbidden(_))) => {
                self.logs = None;
                self.disable(Feature::InputLog);
            }
            Some(Err(error)) => return Err(error),
            Some(Ok(logged)) => {
                if logged.uploaded > 0 || logged.downloaded > 0 || logged.cleared {
                    tracing::info!(?logged, "输入日志同步");
                }
            }
            None => {}
        }
        Ok((outcome, config))
    }

    fn disable(&mut self, feature: Feature) {
        tracing::warn!(feature = feature.as_str(), "服务器上没开这项功能，停止同步");
        if !self.disabled.contains(&feature) {
            self.disabled.push(feature);
        }
    }
}
```

`src/data_sync/mod.rs`：`open_jobs` 末尾的 `Ok(Jobs { … })`（第 106–110 行）替换为：

```rust
    Ok(Jobs {
        learning: config.sync_learning.then_some(learning),
        settings: config.sync_config.then_some(settings),
        logs,
        disabled: Vec::new(),
    })
```

`run` 整个函数（第 113–150 行）替换为：

```rust
fn run(shared: &Shared, mut jobs: Jobs) {
    let mut retry = Duration::from_secs(1);
    while !shared.stop.load(Ordering::Relaxed) {
        let result = jobs.cycle();
        let delay = {
            let mut status = lock(&shared.status);
            status.disabled.clone_from(&jobs.disabled);
            match result {
                Ok((outcome, config)) => {
                    if outcome.pushed > 0
                        || outcome.delivered > 0
                        || config != ConfigOutcome::Unchanged
                    {
                        tracing::info!(?outcome, ?config, "学习数据同步");
                    }
                    status.last_ok_ms = Some(now_ms());
                    status.waiting_for_ime = outcome.waiting;
                    status.error = None;
                    status.unauthorized = false;
                    status.config_conflict |= config == ConfigOutcome::Conflict;
                    retry = Duration::from_secs(1);
                    if outcome.waiting {
                        WAITING_INTERVAL
                    } else {
                        INTERVAL
                    }
                }
                // 令牌被拒：退避没有意义，按最长间隔等；壳看到 unauthorized 会让用户重新登录
                Err(ClientError::Unauthorized) => {
                    tracing::warn!("学习数据同步：令牌被拒");
                    status.unauthorized = true;
                    status.error = Some(ClientError::Unauthorized.to_string());
                    MAX_RETRY
                }
                Err(error) => {
                    tracing::warn!(%error, "学习数据同步失败");
                    status.error = Some(error.to_string());
                    let delay = retry;
                    retry = (retry * 2).min(MAX_RETRY);
                    delay
                }
            }
        };
        wait(shared, delay);
    }
}
```

- [ ] **Step 8: Mac 菜单认 `Status::Disabled`**

`cloud/crates/qingjian-cloud-mac/src/menu/mod.rs` 的 `status_line` 里，`Status::Unauthorized => …` 那一行（第 57 行）之后加一行：

```rust
                Status::Disabled => "跨设备剪贴板在服务器上没开".to_owned(),
```

（Task 5 会整体重写这个文件，这里只为让 workspace 在两次提交之间编得过。）

- [ ] **Step 9: 全 workspace 编译、测试、clippy**

Run:
```bash
cd /Users/liyuqing/sproot/qingjian-mainline/cloud
cargo test -p qingjian-cloud-client
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
```
Expected: client 的测试全部 PASS（含新加的 2 个）；整个 workspace 编译通过、clippy 无警告。

- [ ] **Step 10: 提交**

```bash
cd /Users/liyuqing/sproot/qingjian-mainline
git add cloud/crates/qingjian-cloud-client cloud/crates/qingjian-cloud-mac/src/menu/mod.rs
git commit -m "$(cat <<'EOF'
feat(cloud): 客户端加账号接口，403 当作功能没开停下

Client 加 anonymous 与登录、账号、开关、注销、退出、删账号；ClientError 加 Forbidden（403）与 RateLimited（429），都不重试。
剪贴板两线程收到 403 与 401 一样停 5 分钟、状态报 Disabled，不再把 403 当作拒收丢掉这条剪贴板；
学习数据线程把 403 的那一项从本轮起停掉并记进 DataStatus.disabled，401 记 unauthorized、按最长间隔等。

EOF
)"
```

到这里通知服务端计划可以开始。

---

## Task 3: iOS 桥（qingjian-cloud-bridge）的登录与开关

**Files:**
- Move: `cloud/crates/qingjian-cloud-bridge/src/cloud_config/mod.rs` → `src/cloud_config.rs`（重写）
- Delete: `src/cloud_config/status.rs`、`src/cloud_config/switches.rs`
- Create: `src/account/mod.rs`、`src/account/status.rs`、`src/account/ffi.rs`
- Modify: `src/lib.rs`、`src/session/mod.rs`、`src/session/cloud.rs`、`include/qingjian_bridge.h`
- Test: `tests/cloud_config.rs`（重写）、`tests/settings.rs`（删一个测试）

现状：`cloud.toml` 由构建脚本从开发者本机的 `cloud.local.toml`（带 `qjc_` 令牌）打进键盘包，主 App 第一次打开时拷进 App Group（`SettingsStore.seedCloudConfig`），之后只有设置页经 `qj_cloud_config_write` 改开关；四个开关缺省 `true`。
改完：随包的 `cloud.toml` 只有 `server`，令牌只由桥在登录成功后写入；开关缺省 `false`，跟着服务器上的同意记录写。`CloudConfig` 字段与 `Feature` 的对应：`llm ↔ Llm`、`logs ↔ InputLog`、`sync ↔ Sync`、`clipboard ↔ Clipboard`。

- [ ] **Step 1: 写失败的测试**

`tests/cloud_config.rs` 整个文件替换为：

```rust
//! `cloud.toml`：没登录、旧令牌、格式不对都当离线；登录、改开关、退出登录只动该动的字段。

use std::path::{Path, PathBuf};

use qingjian_cloud_bridge::{AccountStatus, CloudConfig, DEFAULT_SERVER};
use qingjian_cloud_proto::Consents;

fn temp_file(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("qj-cloud-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("cloud.toml")
}

fn load(name: &str, text: &str) -> Option<CloudConfig> {
    let path = temp_file(name);
    std::fs::write(&path, text).unwrap();
    CloudConfig::load(&path)
}

#[test]
fn switches_default_to_off() {
    let config = load(
        "defaults",
        "server = \"https://pinyin.synon.ai/\"\ntoken = \"sjt_t\"\n",
    )
    .unwrap();
    assert!(!config.llm && !config.sync && !config.logs && !config.clipboard);
    assert_eq!(config.llm_base_url(), "https://pinyin.synon.ai/v1");
    assert!(!CloudConfig::default().clipboard);
}

#[test]
fn switches_are_respected() {
    let config = load("switches", "server = \"s\"\ntoken = \"sjt_t\"\nllm = true\n").unwrap();
    assert!(config.llm && !config.sync);
    assert_eq!(config.consents(), Consents { llm: true, ..Consents::default() });
}

#[test]
fn missing_or_old_token_or_bad_toml_means_offline() {
    assert!(load("no-token", "server = \"s\"\n").is_none());
    assert!(load("blank", "server = \"s\"\ntoken = \"  \"\n").is_none());
    assert!(load("old", "server = \"s\"\ntoken = \"qjc_old\"\nsync = true\n").is_none());
    assert!(load("bad", "server = ").is_none());
    assert!(CloudConfig::load(Path::new("/nonexistent/cloud.toml")).is_none());
}

#[test]
fn store_session_writes_server_token_and_consents() {
    let path = temp_file("session");
    std::fs::write(&path, "server = \"https://example.com\"\n").unwrap();
    let consents = Consents {
        sync: true,
        ..Consents::default()
    };
    CloudConfig::store_session(&path, "https://example.com", "sjt_abc", consents).unwrap();
    let config = CloudConfig::load(&path).unwrap();
    assert_eq!(config.server, "https://example.com");
    assert_eq!(config.token, "sjt_abc");
    assert!(config.sync && !config.clipboard && !config.logs && !config.llm);
}

#[test]
fn store_consents_keeps_server_and_token() {
    let path = temp_file("consents");
    CloudConfig::store_session(&path, "https://example.com", "sjt_abc", Consents::default())
        .unwrap();
    let consents = Consents {
        clipboard: true,
        input_log: true,
        ..Consents::default()
    };
    CloudConfig::store_consents(&path, consents).unwrap();
    let config = CloudConfig::read(&path).unwrap();
    assert_eq!(config.token, "sjt_abc");
    assert_eq!(config.server, "https://example.com");
    assert!(config.clipboard && config.logs && !config.sync);
}

#[test]
fn clear_session_drops_token_and_switches_but_keeps_server() {
    let path = temp_file("clear");
    let all = Consents {
        clipboard: true,
        sync: true,
        input_log: true,
        llm: true,
    };
    CloudConfig::store_session(&path, "https://example.com", "sjt_abc", all).unwrap();
    CloudConfig::clear_session(&path).unwrap();
    let config = CloudConfig::read(&path).unwrap();
    assert!(config.token.is_empty());
    assert_eq!(config.consents(), Consents::default());
    assert_eq!(config.server, "https://example.com");
    assert!(CloudConfig::load(&path).is_none());
    // 没有文件时退出登录也算成功
    assert!(CloudConfig::clear_session(&temp_file("clear-missing")).is_ok());
}

#[test]
fn status_without_token_stays_offline() {
    // 不写文件：没登录时只读文件、不联网
    let status = AccountStatus::load(&temp_file("status"));
    assert!(!status.signed_in);
    assert_eq!(status.server, DEFAULT_SERVER);
    assert!(status.error.is_none());
    assert!(status.sessions.is_empty() && status.identities.is_empty());
    let json = serde_json::to_value(&status).unwrap();
    assert_eq!(json["consents"]["input_log"], false);
    assert_eq!(json["signed_in"], false);
    assert!(json.get("token").is_none());
}
```

`tests/settings.rs`：第 5 行改成

```rust
use qingjian_cloud_bridge::{CloudConfig, Session, Settings};
```

并删掉整个 `switches_keep_server_and_token` 测试（第 109–130 行，从 `#[test]` 到文件末尾的 `}`）。开关回写由上面的 `store_consents_keeps_server_and_token` 覆盖。

- [ ] **Step 2: 跑测试，确认失败**

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud && cargo test -p qingjian-cloud-bridge --test cloud_config`
Expected: 编译失败，`unresolved imports qingjian_cloud_bridge::AccountStatus, qingjian_cloud_bridge::DEFAULT_SERVER`，以及 `no function or associated item named store_session`。

- [ ] **Step 3: 重写 `cloud_config`**

```bash
cd /Users/liyuqing/sproot/qingjian-mainline/cloud/crates/qingjian-cloud-bridge/src
git mv cloud_config/mod.rs cloud_config.rs
git rm cloud_config/status.rs cloud_config/switches.rs
```

`src/cloud_config.rs` 整个文件替换为：

```rust
//! 键盘连青简 Cloud 的配置（`cloud.toml`，在 App Group 里）：服务器地址、登录得到的会话令牌与四个功能开关。
//! 地址构建时写进随包的种子；令牌只由登录写入（见 `account`），退出登录、删账号时清空；
//! 开关跟着服务器上的同意记录走，缺省全关。没有这个文件、地址为空或没登录，键盘就完全离线。

use std::path::Path;

use qingjian_cloud_proto::{Consents, TOKEN_PREFIX};
use serde::{Deserialize, Serialize};

/// `cloud.toml` 里没写地址时用的服务器。
pub const DEFAULT_SERVER: &str = "https://pinyin.synon.ai";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct CloudConfig {
    /// 服务器地址，构建时写进随包的 `cloud.toml`。
    pub server: String,

    /// 登录得到的会话令牌（`sjt_` 开头）。旧版的设备令牌（`qjc_`）已作废，当没登录。
    pub token: String,

    /// 用服务器的大模型：润色，以及 `config.toml` 里 `[predict]` 开着时的云联想（服务器上的 `llm`）。
    pub llm: bool,

    /// 记输入日志并上传（服务器上的 `input_log`）。
    pub logs: bool,

    /// 与别的设备同步学习数据与 `config.toml`（服务器上的 `sync`）。
    pub sync: bool,

    /// 跨设备剪贴板（服务器上的 `clipboard`）。
    pub clipboard: bool,
}

impl CloudConfig {
    /// 键盘用：读不了、格式不对、没地址或没登录都当没配置，键盘照常离线用。
    pub fn load(path: &Path) -> Option<Self> {
        let config = Self::read(path)?;
        (!config.server.trim().is_empty() && config.signed_in()).then_some(config)
    }

    /// 原样读出（没登录也读），读不了返回 `None`。
    pub fn read(path: &Path) -> Option<Self> {
        let text = std::fs::read_to_string(path).ok()?;
        toml::from_str(&text)
            .inspect_err(|error| tracing::warn!(%error, "cloud.toml 格式不对，按离线用"))
            .ok()
    }

    pub fn signed_in(&self) -> bool {
        self.token.trim().starts_with(TOKEN_PREFIX)
    }

    /// 去掉末尾的 `/`；没写地址时用 [`DEFAULT_SERVER`]。
    pub fn server_or_default(&self) -> String {
        let server = self.server.trim().trim_end_matches('/');
        if server.is_empty() {
            DEFAULT_SERVER.to_owned()
        } else {
            server.to_owned()
        }
    }

    pub fn consents(&self) -> Consents {
        Consents {
            clipboard: self.clipboard,
            sync: self.sync,
            input_log: self.logs,
            llm: self.llm,
        }
    }

    pub fn set_consents(&mut self, consents: Consents) {
        self.clipboard = consents.clipboard;
        self.sync = consents.sync;
        self.logs = consents.input_log;
        self.llm = consents.llm;
    }

    /// 登录成功：写入地址、令牌与服务器上的开关。
    pub fn store_session(
        path: &Path,
        server: &str,
        token: &str,
        consents: Consents,
    ) -> Result<(), String> {
        let mut config = Self::read(path).unwrap_or_default();
        config.server = server.to_owned();
        config.token = token.to_owned();
        config.set_consents(consents);
        config.save(path)
    }

    /// 服务器上的开关变了：只改开关，地址与令牌不动。
    pub fn store_consents(path: &Path, consents: Consents) -> Result<(), String> {
        let mut config = Self::read(path).unwrap_or_default();
        config.set_consents(consents);
        config.save(path)
    }

    /// 退出登录、删账号、令牌失效：清掉令牌与开关，地址留着。没有文件也算成功。
    pub fn clear_session(path: &Path) -> Result<(), String> {
        let Some(mut config) = Self::read(path) else {
            return Ok(());
        };
        config.token.clear();
        config.set_consents(Consents::default());
        config.save(path)
    }

    /// 整份写回（这份文件只有这几项，不用保留注释）。
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let text = toml::to_string(self).map_err(|e| e.to_string())?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        std::fs::write(path, text).map_err(|e| e.to_string())
    }

    /// 大模型代理的接口地址（OpenAI 兼容，不含 `/chat/completions`）。
    pub fn llm_base_url(&self) -> String {
        format!("{}/v1", self.server_or_default())
    }
}
```

- [ ] **Step 4: 新建 `account` 模块**

`src/account/mod.rs`：

```rust
//! 主 App「账号」页背后的操作：登录（Apple、邮箱两步）、看账号、改功能开关、注销设备、退出登录、删账号。
//! 都是阻塞的网络请求，Swift 在后台调。会话令牌只在 `cloud.toml` 与这里之间流转，不交给 Swift。

mod ffi;
mod status;

use std::path::Path;

use qingjian_cloud_client::{Client, ClientError};
use qingjian_cloud_proto::{
    AppleClient, AppleSignIn, Device, EmailVerify, Feature, Platform, SessionGrant,
};

use crate::cloud_config::CloudConfig;

pub use self::status::AccountStatus;

/// 设备名为空时报给服务端的名字。
const FALLBACK_DEVICE: &str = "iPhone";

pub fn sign_in_apple(
    path: &Path,
    identity_token: &str,
    authorization_code: &str,
    nonce: &str,
    device_name: &str,
) -> Result<(), String> {
    let server = server(path);
    let request = AppleSignIn {
        identity_token: identity_token.to_owned(),
        authorization_code: authorization_code.to_owned(),
        nonce: nonce.to_owned(),
        client: AppleClient::Ios,
        device: device(device_name),
        challenge: None,
    };
    let grant = Client::anonymous(&server)
        .sign_in_apple(&request)
        .map_err(|error| match error {
            ClientError::Unauthorized => "Apple 登录没有通过验证，请重试".to_owned(),
            other => message(&other),
        })?;
    finish(path, &server, &grant)
}

pub fn email_start(path: &Path, email: &str) -> Result<(), String> {
    Client::anonymous(&server(path))
        .email_start(email.trim())
        .map_err(|error| match error {
            ClientError::Rejected {
                status: 400 | 422, ..
            } => "邮箱地址不对，检查后再试".to_owned(),
            other => message(&other),
        })
}

pub fn email_verify(
    path: &Path,
    email: &str,
    code: &str,
    device_name: &str,
) -> Result<(), String> {
    let server = server(path);
    let request = EmailVerify {
        email: email.trim().to_owned(),
        code: code.trim().to_owned(),
        device: device(device_name),
        challenge: None,
    };
    let grant = Client::anonymous(&server)
        .email_verify(&request)
        .map_err(|error| match error {
            ClientError::Unauthorized => "验证码不对或已过期".to_owned(),
            other => message(&other),
        })?;
    finish(path, &server, &grant)
}

/// 先改服务器上的同意记录，成功后把服务器回的四项开关写回 `cloud.toml`。
pub fn set_consent(path: &Path, feature: Feature, enabled: bool) -> Result<(), String> {
    let config = signed_in(path)?;
    match client(&config).put_consent(feature, enabled) {
        Ok(consents) => CloudConfig::store_consents(path, consents),
        Err(error) => Err(expired(path, &error)),
    }
}

pub fn revoke_session(path: &Path, id: i64) -> Result<(), String> {
    let config = signed_in(path)?;
    client(&config)
        .revoke_session(id)
        .map_err(|error| expired(path, &error))
}

/// 本机总会退出（清掉令牌与开关）；服务器上没注销掉只记日志，可以从别的设备再注销这台。
pub fn sign_out(path: &Path) -> Result<(), String> {
    if let Ok(config) = signed_in(path)
        && let Err(error) = client(&config).sign_out()
    {
        tracing::warn!(%error, "服务器上退出登录失败，本机照样退出");
    }
    CloudConfig::clear_session(path)
}

/// 服务器删成功才清本机令牌；令牌已经失效时也清掉，免得界面一直显示已登录。
pub fn delete_account(path: &Path) -> Result<(), String> {
    let config = signed_in(path)?;
    match client(&config).delete_account() {
        Ok(()) => CloudConfig::clear_session(path),
        Err(error) => Err(expired(path, &error)),
    }
}

/// 登录拿到令牌：先问一次服务器上的开关（老用户在别的设备上开过的照旧开，取不到就全关），
/// 连同地址、令牌写进 `cloud.toml`。换了账号，旧的同步进度作废，删掉重来。
fn finish(path: &Path, server: &str, grant: &SessionGrant) -> Result<(), String> {
    let consents = Client::new(server, &grant.token)
        .account()
        .map(|account| account.consents)
        .inspect_err(|error| tracing::warn!(%error, "登录后取开关失败，先全关"))
        .unwrap_or_default();
    reset_sync_state(path);
    CloudConfig::store_session(path, server, &grant.token, consents)
}

/// 键盘的同步进度（学习数据基线、剪贴板进度）在 `cloud.toml` 同目录的 `cloud/` 下
/// （开了完全访问时学习数据目录就是 App Group 目录）。基线留着的话，新账号服务器上是空的，
/// 算出来「别的设备的增量」是负的，会把本机学到的减掉。
fn reset_sync_state(path: &Path) {
    let Some(dir) = path.parent().map(|dir| dir.join("cloud")) else {
        return;
    };
    match std::fs::remove_dir_all(&dir) {
        Ok(()) => tracing::info!("换了账号，清掉旧的同步进度"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => tracing::warn!(%error, "旧的同步进度删不掉"),
    }
}

fn server(path: &Path) -> String {
    CloudConfig::read(path)
        .unwrap_or_default()
        .server_or_default()
}

fn signed_in(path: &Path) -> Result<CloudConfig, String> {
    CloudConfig::read(path)
        .filter(CloudConfig::signed_in)
        .ok_or_else(|| "还没有登录".to_owned())
}

fn client(config: &CloudConfig) -> Client {
    Client::new(&config.server_or_default(), &config.token)
}

fn device(name: &str) -> Device {
    let name = name.trim();
    Device {
        name: if name.is_empty() {
            FALLBACK_DEVICE.to_owned()
        } else {
            name.to_owned()
        },
        platform: Platform::Ios,
    }
}

/// 令牌失效（被别的设备注销、账号删了）：清掉本机令牌，界面回到未登录。
fn expired(path: &Path, error: &ClientError) -> String {
    if matches!(error, ClientError::Unauthorized) {
        if let Err(reason) = CloudConfig::clear_session(path) {
            tracing::warn!(%reason, "清令牌失败");
        }
        return "登录已失效，请重新登录".to_owned();
    }
    message(error)
}

/// 给用户看的失败原因。
fn message(error: &ClientError) -> String {
    match error {
        ClientError::Unreachable(_) | ClientError::Io(_) => "连不上服务器，检查网络后再试".to_owned(),
        ClientError::Unauthorized => "登录已失效，请重新登录".to_owned(),
        ClientError::Forbidden(_) => "服务器不允许这个操作".to_owned(),
        ClientError::RateLimited => "操作太频繁，请稍后再试".to_owned(),
        ClientError::Rejected { status, .. } => format!("服务器拒绝了请求（{status}）"),
        ClientError::BadResponse(_) => "服务器的回应看不懂，请升级 App".to_owned(),
    }
}
```

`src/account/status.rs`：

```rust
//! 账号页显示的内容：连哪台服务器、登没登录、四个开关、登录方式与设备。令牌不在里面。

use std::path::Path;

use qingjian_cloud_client::{Client, ClientError};
use qingjian_cloud_proto::{Consents, IdentityInfo, SessionInfo};
use serde::Serialize;

use crate::cloud_config::CloudConfig;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AccountStatus {
    /// 服务器地址，只读显示。
    pub server: String,

    pub signed_in: bool,

    /// 服务器上的开关；没登录或这次取不到时照 `cloud.toml`。
    pub consents: Consents,

    pub identities: Vec<IdentityInfo>,

    pub sessions: Vec<SessionInfo>,

    /// 这次没能从服务器取到账号的原因。
    pub error: Option<String>,
}

impl AccountStatus {
    /// 没登录时只读文件、不联网。登录了就问一次服务器，开关和本机文件不一样时以服务器为准写回
    /// （键盘按 `cloud.toml` 的修改时间重开会话，所以一样时不写）；令牌失效就清掉，显示未登录。
    pub fn load(path: &Path) -> Self {
        let config = CloudConfig::read(path).unwrap_or_default();
        let mut status = Self::offline(&config);
        if !status.signed_in {
            return status;
        }
        match Client::new(&status.server, &config.token).account() {
            Ok(account) => {
                if account.consents != config.consents()
                    && let Err(reason) = CloudConfig::store_consents(path, account.consents)
                {
                    tracing::warn!(%reason, "开关写回 cloud.toml 失败");
                }
                status.consents = account.consents;
                status.identities = account.identities;
                status.sessions = account.sessions;
            }
            Err(ClientError::Unauthorized) => {
                if let Err(reason) = CloudConfig::clear_session(path) {
                    tracing::warn!(%reason, "清令牌失败");
                }
                status = Self::offline(&CloudConfig::read(path).unwrap_or_default());
                status.error = Some("登录已失效，请重新登录".to_owned());
            }
            Err(error) => status.error = Some(super::message(&error)),
        }
        status
    }

    /// 只看本机文件。
    pub fn offline(config: &CloudConfig) -> Self {
        Self {
            server: config.server_or_default(),
            signed_in: config.signed_in(),
            consents: config.consents(),
            identities: Vec::new(),
            sessions: Vec::new(),
            error: None,
        }
    }
}
```

`src/account/ffi.rs`：

```rust
//! 账号的 C 接口（主 App 用），与 `include/qingjian_bridge.h` 一一对应。都是阻塞的网络请求，Swift 在后台调。
//! `path` 是 App Group 里的 `cloud.toml`。返回值：状态返回 JSON（参数无效时为空）；操作成功返回空，失败返回给用户看的原因。
//! 都用 `qj_string_free` 释放。

use std::ffi::c_char;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::ptr;

use qingjian_cloud_proto::Feature;

use super::AccountStatus;
use crate::{owned, path_arg};

/// 账号页的 JSON（[`AccountStatus`]）。没登录时不联网。
///
/// # Safety
/// `path` 是有效的 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_account_status(path: *const c_char) -> *mut c_char {
    let Some(path) = (unsafe { path_arg(path) }) else {
        return ptr::null_mut();
    };
    catch_unwind(AssertUnwindSafe(|| {
        serde_json::to_string(&AccountStatus::load(Path::new(path))).ok()
    }))
    .ok()
    .flatten()
    .map_or(ptr::null_mut(), |json| owned(&json))
}

/// Apple 登录：`nonce` 是原始值（交给 Apple 的是它的 SHA-256 十六进制），`device` 是设备名（可为空）。
///
/// # Safety
/// 前四个参数是有效的 UTF-8 C 字符串，`device` 为空或同上。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_account_sign_in_apple(
    path: *const c_char,
    identity_token: *const c_char,
    authorization_code: *const c_char,
    nonce: *const c_char,
    device: *const c_char,
) -> *mut c_char {
    let (Some(path), Some(identity_token), Some(authorization_code), Some(nonce)) = (
        unsafe { path_arg(path) },
        unsafe { path_arg(identity_token) },
        unsafe { path_arg(authorization_code) },
        unsafe { path_arg(nonce) },
    ) else {
        return owned("参数无效");
    };
    let device = unsafe { path_arg(device) }.unwrap_or_default();
    outcome(|| {
        super::sign_in_apple(
            Path::new(path),
            identity_token,
            authorization_code,
            nonce,
            device,
        )
    })
}

/// 给邮箱发验证码。
///
/// # Safety
/// 两个参数都是有效的 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_account_email_start(
    path: *const c_char,
    email: *const c_char,
) -> *mut c_char {
    let (Some(path), Some(email)) = (unsafe { path_arg(path) }, unsafe { path_arg(email) }) else {
        return owned("参数无效");
    };
    outcome(|| super::email_start(Path::new(path), email))
}

/// 邮箱加验证码登录。
///
/// # Safety
/// 前三个参数是有效的 UTF-8 C 字符串，`device` 为空或同上。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_account_email_verify(
    path: *const c_char,
    email: *const c_char,
    code: *const c_char,
    device: *const c_char,
) -> *mut c_char {
    let (Some(path), Some(email), Some(code)) = (
        unsafe { path_arg(path) },
        unsafe { path_arg(email) },
        unsafe { path_arg(code) },
    ) else {
        return owned("参数无效");
    };
    let device = unsafe { path_arg(device) }.unwrap_or_default();
    outcome(|| super::email_verify(Path::new(path), email, code, device))
}

/// 开关一项功能（`clipboard` / `sync` / `input_log` / `llm`），成功后写回 `cloud.toml` 的开关。
///
/// # Safety
/// 两个字符串参数都是有效的 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_account_set_consent(
    path: *const c_char,
    feature: *const c_char,
    enabled: bool,
) -> *mut c_char {
    let (Some(path), Some(feature)) = (unsafe { path_arg(path) }, unsafe {
        path_arg(feature).and_then(Feature::parse)
    }) else {
        return owned("参数无效");
    };
    outcome(|| super::set_consent(Path::new(path), feature, enabled))
}

/// 注销本账号的某台设备。
///
/// # Safety
/// `path` 是有效的 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_account_revoke_session(
    path: *const c_char,
    session_id: i64,
) -> *mut c_char {
    let Some(path) = (unsafe { path_arg(path) }) else {
        return owned("参数无效");
    };
    outcome(|| super::revoke_session(Path::new(path), session_id))
}

/// 退出登录：本机总会退出，服务器上没注销掉只记日志。
///
/// # Safety
/// `path` 是有效的 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_account_sign_out(path: *const c_char) -> *mut c_char {
    let Some(path) = (unsafe { path_arg(path) }) else {
        return owned("参数无效");
    };
    outcome(|| super::sign_out(Path::new(path)))
}

/// 删账号：服务器删成功才清本机令牌。
///
/// # Safety
/// `path` 是有效的 UTF-8 C 字符串。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn qj_account_delete(path: *const c_char) -> *mut c_char {
    let Some(path) = (unsafe { path_arg(path) }) else {
        return owned("参数无效");
    };
    outcome(|| super::delete_account(Path::new(path)))
}

/// 成功返回空，失败返回原因；panic 折成一句通用的话（穿过 `extern "C"` 会直接 abort）。
fn outcome(f: impl FnOnce() -> Result<(), String>) -> *mut c_char {
    match catch_unwind(AssertUnwindSafe(f)) {
        Ok(Ok(())) => ptr::null_mut(),
        Ok(Err(reason)) => owned(&reason),
        Err(_) => owned("出错了，请重试"),
    }
}
```

- [ ] **Step 5: 改 `src/lib.rs`**

1. `mod` 列表第一行前加 `mod account;`（结果是 `mod account; mod clipboard; mod cloud_config; …`）。
2. 第 20 行 `pub use self::cloud_config::{CloudConfig, CloudStatus, CloudSwitches};` 替换为两行（放在 `pub use self::clipboard…` 之前保持字母序）：

```rust
pub use self::account::AccountStatus;
pub use self::clipboard::{ClipOffer, Clipboard};
pub use self::cloud_config::{CloudConfig, DEFAULT_SERVER};
```

（原来的 `pub use self::clipboard::{ClipOffer, Clipboard};` 那一行删掉，避免重复。）
3. 删掉 `qj_cloud_config_read` 与 `qj_cloud_config_write` 两个函数连同它们的文档注释（第 446–480 行，从 `/// 读 \`cloud.toml\` 给设置页` 到 `qj_cloud_config_write` 的结尾 `}`）。账号页改用 `qj_account_status`，开关改用 `qj_account_set_consent`。
4. 文件末尾两个辅助函数改成 crate 内可见（`account/ffi.rs` 要用）：

```rust
/// 文字里不会有 NUL（候选与拼音都来自词库），万一有就截到 NUL 前。
pub(crate) fn owned(text: &str) -> *mut c_char {
    let text = text.split('\0').next().unwrap_or_default();
    CString::new(text).map_or(ptr::null_mut(), CString::into_raw)
}

pub(crate) unsafe fn path_arg<'a>(raw: *const c_char) -> Option<&'a str> {
    if raw.is_null() {
        return None;
    }
    unsafe { CStr::from_ptr(raw) }.to_str().ok()
}
```

- [ ] **Step 6: 键盘会话按服务器的两个开关分开起同步**

`src/session/mod.rs` 的输入日志条件（第 78–85 行）替换为：

```rust
        // 只在登录了且开了「上传输入日志」时记日志：离线或没开的用户，输入不落任何日志
        if let (Some(dir), Some(cloud)) = (user_dir, &cloud)
            && cloud.logs
        {
            engine =
                engine.with_input_logger(Box::new(InputLog::open(dir.join("input-log.jsonl"))));
        }
```

`src/session/cloud.rs` 的 `connect` 里 DataSync 那段（第 30–46 行，`if let (true, Some(user_dir)) = (cloud.sync, &self.user_dir) {` 起到这个 `if` 的 `}`）替换为：

```rust
        // 学习数据与设置跟 `sync` 走，输入日志跟 `logs` 走：服务器上是两个独立的开关
        if let (true, Some(user_dir)) = (cloud.sync || cloud.logs, &self.user_dir) {
            let started = DataSync::start(DataSyncConfig {
                server: cloud.server.clone(),
                token: cloud.token.clone(),
                ime_dir: user_dir.clone(),
                state_dir: user_dir.join("cloud"),
                sync_learning: cloud.sync,
                // config.toml 与 Mac 同步（模糊音、短语、词库开关……）；输入日志上传给纠错闭环，别的设备的不下载
                sync_logs: cloud.logs,
                log_download_dir: None,
                sync_config: cloud.sync,
            });
            match started {
                Ok(sync) => self.data_sync = Some(sync),
                Err(error) => tracing::warn!(%error, "学习数据同步启动失败"),
            }
        }
```

- [ ] **Step 7: 改头文件**

`include/qingjian_bridge.h` 里这一段（第 64–67 行）：

```c
// cloud.toml：读返回服务器地址、是否已连与各开关（不含令牌）；写只改开关，地址与令牌不动。键盘下次弹出时生效。
char *qj_cloud_config_read(const char *path);
char *qj_cloud_config_write(const char *path, const char *json);
```

替换为：

```c
// 账号（主 App 用）：path 是 App Group 里的 cloud.toml；都是阻塞的网络请求，在后台线程调。令牌只在 cloud.toml 与桥之间流转。
// status 返回 JSON（参数无效时为 NULL，没登录时不联网）；其余成功返回 NULL，失败返回给用户看的原因。键盘下次弹出时按新的 cloud.toml 重连。
char *qj_account_status(const char *path);
// nonce 是原始值（交给 Apple 的是它的 SHA-256 十六进制）；device 是设备名，可为 NULL。
char *qj_account_sign_in_apple(const char *path, const char *identity_token,
                               const char *authorization_code, const char *nonce,
                               const char *device);
char *qj_account_email_start(const char *path, const char *email);
char *qj_account_email_verify(const char *path, const char *email, const char *code,
                              const char *device);
// feature：clipboard / sync / input_log / llm；成功后同时写回 cloud.toml 的开关。
char *qj_account_set_consent(const char *path, const char *feature, bool enabled);
char *qj_account_revoke_session(const char *path, int64_t session_id);
// 退出登录：本机总会退出（清令牌与开关），服务器上没注销掉只记日志。
char *qj_account_sign_out(const char *path);
// 删账号：服务器删成功才清本机令牌。
char *qj_account_delete(const char *path);
```

- [ ] **Step 8: 跑测试，确认通过**

Run:
```bash
cd /Users/liyuqing/sproot/qingjian-mainline/cloud
cargo test -p qingjian-cloud-bridge
cargo clippy -p qingjian-cloud-bridge --all-targets -- -D warnings
```
Expected: `tests/cloud_config.rs` 7 个测试 PASS，其余测试（`session.rs` 没有 `QINGJIAN_DATA` 时跳过）照常 PASS；clippy 无警告。
再确认符号导出：`cargo build -p qingjian-cloud-bridge && nm -gU target/debug/libqingjian_cloud_bridge.a 2>/dev/null | grep qj_account_`，Expected: 列出 8 个 `_qj_account_*`。

- [ ] **Step 9: 提交**

```bash
cd /Users/liyuqing/sproot/qingjian-mainline
git add cloud/crates/qingjian-cloud-bridge
git commit -m "$(cat <<'EOF'
feat(cloud): iOS 桥加账号登录，cloud.toml 开关缺省关

C ABI 加 Apple / 邮箱登录、账号状态、开关、注销设备、退出登录、删账号；登录成功把地址与令牌写进 App Group 的 cloud.toml，令牌不出桥。
开关跟服务器上的同意记录走，缺省全关；旧的 qjc_ 令牌当没登录，键盘离线。换账号时删掉旧的同步进度，免得旧基线把本机学习数据减掉。
DataSync 按 sync 与 logs 两个开关分开起；去掉 qj_cloud_config_read/write，iOS 工程要到下一个提交才能再编过。

EOF
)"
```

---

## Task 4: iOS App 的「账号」页

**Files:**
- Modify: `cloud/ios/project.yml`、`cloud/ios/App/App.entitlements`、`cloud/ios/App/SetupView.swift`、`cloud/ios/App/Settings/SettingsBridge.swift`、`cloud/ios/App/Settings/SettingsStore.swift`
- Delete: `cloud/ios/App/Settings/CloudSettings.swift`、`cloud/ios/App/Settings/CloudSettingsView.swift`
- Create: `cloud/ios/App/Account/{AccountState,Consents,CloudFeature,AccountIdentity,AccountDevice,Nonce,AccountBridge,AccountStore,AccountView,SignInSections,DeviceRow}.swift`
- Modify: `cloud/ios/scripts/build-bridge.sh`、`cloud/ios/scripts/install-device.sh`、`cloud/ios/cloud.example.toml`
- Test: 模拟器编译 + 真机步骤（Swift 侧没有单测 target，桥的逻辑已在 Task 3 单测）

键盘不改逻辑：`UserData.resolve` 照旧找 App Group 的 `cloud.toml`（没有时用随包的），`Engine` 把路径交给 `qj_session_open`，桥里 `CloudConfig::load` 在没登录、旧 `qjc_` 令牌时返回 `None`，会话不建任何客户端、不记输入日志；`Keyboard/Sources` 里没有任何 `URLSession`（已 grep 确认）。所以没登录时键盘完全离线。`KeyboardViewController.currentSignature` 按 `cloud.toml` 修改时间重开引擎，登录、改开关、退出后键盘下次弹出就用新配置。

- [ ] **Step 1: 先确认现在的工程编不过（Task 3 删了接口）**

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud/ios && xcodegen generate && xcodebuild -scheme QingjianCloud -destination 'platform=iOS Simulator,name=iPhone 17e' build 2>&1 | tail -5`
Expected: `** BUILD FAILED **`，错误是 `cannot find 'qj_cloud_config_read' in scope`（`SettingsBridge.swift`）。

- [ ] **Step 2: 工程与权限**

`project.yml` 里 QingjianCloud target 的 `entitlements.properties`（现在只有 `com.apple.security.application-groups` 一行）替换为：

```yaml
      properties:
        com.apple.security.application-groups: [group.app.qingjian.cloud]
        # 账号页的「通过 Apple 登录」；服务端核对 identity_token 的 aud = app.qingjian.cloud
        com.apple.developer.applesignin: [Default]
```

`App/App.entitlements` 整个文件替换为（xcodegen 也会按上面的 properties 重写它，提交的版本要一致）：

```xml
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
	<key>com.apple.developer.applesignin</key>
	<array>
		<string>Default</string>
	</array>
	<key>com.apple.security.application-groups</key>
	<array>
		<string>group.app.qingjian.cloud</string>
	</array>
</dict>
</plist>
```

- [ ] **Step 3: 构建脚本只写服务器地址**

`scripts/build-bridge.sh`：第 3 行注释改成

```bash
# 再把产品数据（dict.qj、lm.qj、领域词库）与只写了服务器地址的 cloud.toml 放进 Keyboard/Data/。Xcode 工程的 preBuildScript 会调它，也可手动跑。
```

第 4 行用法注释后面追加一句 `；服务器地址缺省 https://pinyin.synon.ai，可用 QJ_SERVER 覆盖。`
末尾 `cloud.local.toml` 那一段（从 `# 青简 Cloud 的连接配置（服务器地址 + 这台设备的令牌）` 到对应的 `fi`）替换为：

```bash
# 青简 Cloud 只在构建时定服务器地址；令牌由主 App 的账号页登录后写进 App Group 的 cloud.toml，不进安装包。
# 随包的这份只作种子：主 App 第一次打开时拷进 App Group
server="${QJ_SERVER:-https://pinyin.synon.ai}"
seed="$ios_dir/Keyboard/Data/cloud.toml"
printf 'server = "%s"\n' "$server" > "$seed.tmp"
if cmp -s "$seed.tmp" "$seed"; then
  rm "$seed.tmp"
else
  mv "$seed.tmp" "$seed"
fi
```

`scripts/install-device.sh`：第 15–16 行注释里的 `与 cloud.toml` 保留；把

```bash
if [[ -f cloud.local.toml && ! -f "$app/PlugIns/Keyboard.appex/Data/cloud.toml" ]]; then
  echo "包里没有 cloud.toml，装上也连不了青简 Cloud" >&2
  exit 1
fi
```

替换为：

```bash
seed="$app/PlugIns/Keyboard.appex/Data/cloud.toml"
[[ -f "$seed" ]] || { echo "包里没有 cloud.toml，账号页不知道连哪台服务器" >&2; exit 1; }
if grep -q '^token' "$seed"; then
  echo "包里的 cloud.toml 带着令牌，不能装出去（令牌只由账号页登录写入）" >&2
  exit 1
fi
```

`cloud.example.toml` 整个文件替换为：

```toml
# App Group 里 cloud.toml 的样子（键盘与主 App 共用，由桥读写）。不用复制、不用手填：
# 构建脚本只往安装包里写 server（环境变量 QJ_SERVER 可换地址），主 App 第一次打开时拷进 App Group；
# 账号页登录后由桥写入 token 与四个开关，退出登录、删账号时清掉。

server = "https://pinyin.synon.ai"

# 以下由账号页按服务器上的开关写入，缺省全关
# 跨设备剪贴板
clipboard = false
# 与 Mac 等设备同步学习数据与设置
sync = false
# 记输入日志并上传，服务器据此纠错、补热词、调词频
logs = false
# 用服务器的大模型：润色，以及键盘设置里开了「云联想」时组字补候选
llm = false
```

开发者本机的 `cloud/ios/cloud.local.toml` 带着已作废的旧令牌，提醒用户自己删掉（`.gitignore` 里那一行保留，防止误提交）。

- [ ] **Step 4: 删旧页面，改设置的桥与状态**

```bash
cd /Users/liyuqing/sproot/qingjian-mainline/cloud/ios
git rm App/Settings/CloudSettings.swift App/Settings/CloudSettingsView.swift
mkdir -p App/Account
```

`App/Settings/SettingsBridge.swift` 整个文件替换为：

```swift
// 调 Rust 桥读写 config.toml；JSON 进出，失败的原因是给用户看的中文。账号接口在 Account/AccountBridge.swift。

import Foundation
import QingjianBridge

enum SettingsBridge {
    static func readSettings(config: URL, dicts: URL) -> KeyboardSettings? {
        let raw = config.path.withCString { c in dicts.path.withCString { qj_settings_read(c, $0) } }
        return decode(take(raw))
    }

    /// 成功返回 nil，失败返回原因。
    static func writeSettings(_ settings: KeyboardSettings, config: URL) -> String? {
        guard let json = encode(settings) else { return "设置编码失败" }
        return take(config.path.withCString { c in json.withCString { qj_settings_write(c, $0) } })
    }

    /// 取走桥返回的字符串并释放。
    static func take(_ raw: UnsafeMutablePointer<CChar>?) -> String? {
        guard let raw else { return nil }
        defer { qj_string_free(raw) }
        return String(cString: raw)
    }

    static func decode<T: Decodable>(_ json: String?) -> T? {
        guard let data = json?.data(using: .utf8) else { return nil }
        let decoder = JSONDecoder()
        decoder.keyDecodingStrategy = .convertFromSnakeCase
        return try? decoder.decode(T.self, from: data)
    }

    private static func encode<T: Encodable>(_ value: T) -> String? {
        let encoder = JSONEncoder()
        encoder.keyEncodingStrategy = .convertToSnakeCase
        return (try? encoder.encode(value)).flatMap { String(data: $0, encoding: .utf8) }
    }
}
```

`App/Settings/SettingsStore.swift` 整个文件替换为：

```swift
// 设置页的状态：从 App Group 里的 config.toml 读出，改一项写一次。
// 键盘每次轮询按修改时间重读 config.toml；同一个文件经青简 Cloud 与 Mac 同步，所以打开设置页时重读一遍。

import Foundation
import Observation

@MainActor
@Observable
final class SettingsStore {
    private(set) var settings: KeyboardSettings?

    /// 设置页要弹出来的写入失败原因。
    var error: String?

    /// 签名没带 App Group：主 App 与键盘不共享文件，设置页改不了键盘。
    var available: Bool { SharedStore.directory != nil }

    init() {
        Self.seedCloudConfig()
        reload()
    }

    func reload() {
        guard let config = SharedStore.configFile else { return }
        SharedStore.ensure(SharedStore.directory)
        settings = SettingsBridge.readSettings(config: config, dicts: Self.dictsDirectory)
    }

    /// 改一项就写回；写不进去（如自定义短语不合法）时撤回这次改动，返回原因。
    @discardableResult
    func update(_ change: (inout KeyboardSettings) -> Void) -> String? {
        guard var next = settings, let config = SharedStore.configFile else { return "设置文件不可用" }
        change(&next)
        if let failure = SettingsBridge.writeSettings(next, config: config) { return failure }
        settings = next
        return nil
    }

    /// 随包领域词库在键盘扩展的 Data/dicts 里，主 App 直接读扩展包。
    private static var dictsDirectory: URL {
        Bundle.main.builtInPlugInsURL!
            .appendingPathComponent("Keyboard.appex/Data/dicts", isDirectory: true)
    }

    /// 随包的 cloud.toml 只写了服务器地址（构建时定），共享目录里还没有时拷一份；令牌由账号页登录写入。
    private static func seedCloudConfig() {
        guard let target = SharedStore.cloudFile,
              !FileManager.default.fileExists(atPath: target.path),
              let bundled = Bundle.main.builtInPlugInsURL?
                .appendingPathComponent("Keyboard.appex/Data/cloud.toml"),
              FileManager.default.fileExists(atPath: bundled.path)
        else { return }
        SharedStore.ensure(SharedStore.directory)
        try? FileManager.default.copyItem(at: bundled, to: target)
    }
}
```

- [ ] **Step 5: 账号的数据类型**

`App/Account/AccountState.swift`：

```swift
// 账号页的数据，与桥的 `AccountStatus` JSON 对应（蛇形命名由 SettingsBridge.decode 转换）。令牌不在里面。

import Foundation

struct AccountState: Codable, Equatable {
    var server: String

    var signedIn: Bool

    /// 服务器上的开关；没登录或这次取不到时是 cloud.toml 里的。
    var consents: Consents

    var identities: [AccountIdentity]

    var sessions: [AccountDevice]

    /// 这次没能从服务器取到账号的原因。
    var error: String?
}
```

`App/Account/Consents.swift`：

```swift
// 四项云功能开没开，与协议的 `Consents` 对应。

import Foundation

struct Consents: Codable, Equatable {
    var clipboard: Bool

    var sync: Bool

    var inputLog: Bool

    var llm: Bool

    subscript(feature: CloudFeature) -> Bool {
        get {
            switch feature {
            case .clipboard: clipboard
            case .sync: sync
            case .inputLog: inputLog
            case .llm: llm
            }
        }
        set {
            switch feature {
            case .clipboard: clipboard = newValue
            case .sync: sync = newValue
            case .inputLog: inputLog = newValue
            case .llm: llm = newValue
            }
        }
    }
}
```

`App/Account/CloudFeature.swift`：

```swift
// 要用户单独打开的云功能；rawValue 是协议里的名字（交给桥的 qj_account_set_consent）。

import Foundation

enum CloudFeature: String, CaseIterable, Identifiable {
    case clipboard
    case sync
    case inputLog = "input_log"
    case llm

    var id: String { rawValue }

    var title: String {
        switch self {
        case .clipboard: "跨设备剪贴板"
        case .sync: "同步学习数据与设置"
        case .inputLog: "上传输入日志（服务器据此纠错调频）"
        case .llm: "大模型（润色、云联想）"
        }
    }
}
```

`App/Account/AccountIdentity.swift`：

```swift
// 账号的一种登录方式。

import Foundation

struct AccountIdentity: Codable, Equatable, Hashable {
    /// apple / email。
    var provider: String

    /// 邮箱地址；Apple 没给邮箱时为空。
    var label: String?

    var title: String {
        switch provider {
        case "apple": "Apple"
        case "email": "邮箱"
        default: provider
        }
    }
}
```

`App/Account/AccountDevice.swift`：

```swift
// 账号下的一台已登录设备（会话），时间是 Unix 毫秒。

import Foundation

struct AccountDevice: Codable, Equatable, Identifiable {
    var id: Int64

    var name: String

    /// ios / macos / web。
    var platform: String

    var createdAt: Int64

    var lastSeen: Int64?

    /// 是不是这台 iPhone。
    var current: Bool

    /// 「iOS · 3分钟前活跃」。
    var detail: String {
        let platformTitle = switch platform {
        case "ios": "iOS"
        case "macos": "Mac"
        case "web": "网页"
        default: platform
        }
        let formatter = RelativeDateTimeFormatter()
        formatter.locale = Locale(identifier: "zh_CN")
        if let lastSeen {
            let at = Date(timeIntervalSince1970: Double(lastSeen) / 1000)
            return "\(platformTitle) · \(formatter.localizedString(for: at, relativeTo: .now))活跃"
        }
        let at = Date(timeIntervalSince1970: Double(createdAt) / 1000)
        return "\(platformTitle) · \(formatter.localizedString(for: at, relativeTo: .now))登录"
    }
}
```

`App/Account/Nonce.swift`：

```swift
// Apple 登录的 nonce：请求里给 Apple 的是 SHA-256 十六进制，原始值交给桥，服务端核对 identity_token 里的 nonce 声明。

import CryptoKit
import Foundation

enum Nonce {
    /// 32 字节随机数的十六进制。SystemRandomNumberGenerator 在 Apple 平台上是密码学安全的。
    static func random() -> String {
        var generator = SystemRandomNumberGenerator()
        return (0..<32)
            .map { _ in String(format: "%02x", UInt8.random(in: .min ... .max, using: &generator)) }
            .joined()
    }

    static func sha256Hex(_ text: String) -> String {
        SHA256.hash(data: Data(text.utf8)).map { String(format: "%02x", $0) }.joined()
    }
}
```

- [ ] **Step 6: 账号的桥与状态**

`App/Account/AccountBridge.swift`：

```swift
// 调 Rust 桥的账号接口（qj_account_*）：都是阻塞的网络请求，只在后台任务里调（见 AccountStore）。
// 操作成功返回 nil，失败返回给用户看的原因；令牌留在桥与 cloud.toml 之间，不经过 Swift。

import Foundation
import QingjianBridge

enum AccountBridge {
    static func status(_ file: URL) -> AccountState? {
        SettingsBridge.decode(SettingsBridge.take(file.path.withCString { qj_account_status($0) }))
    }

    static func signInApple(
        _ file: URL, identityToken: String, authorizationCode: String, nonce: String, device: String
    ) -> String? {
        SettingsBridge.take(file.path.withCString { f in
            identityToken.withCString { t in
                authorizationCode.withCString { c in
                    nonce.withCString { n in
                        device.withCString { qj_account_sign_in_apple(f, t, c, n, $0) }
                    }
                }
            }
        })
    }

    static func emailStart(_ file: URL, email: String) -> String? {
        SettingsBridge.take(file.path.withCString { f in
            email.withCString { qj_account_email_start(f, $0) }
        })
    }

    static func emailVerify(_ file: URL, email: String, code: String, device: String) -> String? {
        SettingsBridge.take(file.path.withCString { f in
            email.withCString { e in
                code.withCString { c in
                    device.withCString { qj_account_email_verify(f, e, c, $0) }
                }
            }
        })
    }

    static func setConsent(_ file: URL, feature: String, enabled: Bool) -> String? {
        SettingsBridge.take(file.path.withCString { f in
            feature.withCString { qj_account_set_consent(f, $0, enabled) }
        })
    }

    static func revokeSession(_ file: URL, id: Int64) -> String? {
        SettingsBridge.take(file.path.withCString { qj_account_revoke_session($0, id) })
    }

    static func signOut(_ file: URL) -> String? {
        SettingsBridge.take(file.path.withCString { qj_account_sign_out($0) })
    }

    static func deleteAccount(_ file: URL) -> String? {
        SettingsBridge.take(file.path.withCString { qj_account_delete($0) })
    }
}
```

`App/Account/AccountStore.swift`：

```swift
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
        if let error = next.error { message = error }
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
        if sent { message = "验证码已发出，10 分钟内有效" }
        return sent
    }

    /// 用验证码登录；成功返回 true。
    func emailVerify(_ email: String, code: String) async -> Bool {
        let device = UIDevice.current.name
        let signedIn = await perform({
            AccountBridge.emailVerify($0, email: email, code: code, device: device)
        })
        if signedIn { message = "已登录" }
        return signedIn
    }

    /// 先改界面再问服务器，失败就改回去。
    func setConsent(_ feature: CloudFeature, _ enabled: Bool) async {
        guard let previous = state?.consents else { return }
        state?.consents[feature] = enabled
        let name = feature.rawValue
        let changed = await perform({ AccountBridge.setConsent($0, feature: name, enabled: enabled) })
        if !changed { state?.consents = previous }
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

    /// 在后台跑一次桥的操作；失败把原因放进 message。成功后默认整页重取。
    @discardableResult
    private func perform(
        _ work: @escaping @Sendable (URL) -> String?, refreshAfter: Bool = true
    ) async -> Bool {
        guard let file = SharedStore.cloudFile else {
            message = Self.noGroup
            return false
        }
        busy = true
        let failure = await Task.detached(priority: .userInitiated) { work(file) }.value
        busy = false
        message = failure
        guard failure == nil else { return false }
        if refreshAfter { await refresh() }
        return true
    }
}
```

- [ ] **Step 7: 账号页的视图**

`App/Account/SignInSections.swift`：

```swift
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
```

`App/Account/DeviceRow.swift`：

```swift
// 设备列表的一行：名字、平台、最近活跃；本机标「本机」，别的设备可以注销（二次确认）。

import SwiftUI

struct DeviceRow: View {
    let device: AccountDevice

    let revoke: () -> Void

    @State private var confirming = false

    var body: some View {
        HStack {
            VStack(alignment: .leading, spacing: 2) {
                HStack(spacing: 6) {
                    Text(device.name)
                    if device.current {
                        Text("本机")
                            .font(.caption)
                            .foregroundStyle(.white)
                            .padding(.horizontal, 6)
                            .padding(.vertical, 1)
                            .background(.tint, in: Capsule())
                    }
                }
                Text(device.detail).font(.caption).foregroundStyle(.secondary)
            }
            Spacer()
            if !device.current {
                Button("注销", role: .destructive) { confirming = true }
                    .buttonStyle(.borderless)
            }
        }
        .confirmationDialog("注销「\(device.name)」？", isPresented: $confirming, titleVisibility: .visible) {
            Button("注销", role: .destructive, action: revoke)
        } message: {
            Text("那台设备会退出登录，要用时重新登录。")
        }
    }
}
```

`App/Account/AccountView.swift`：

```swift
// 账号页：没登录时 Apple 与邮箱两种登录；登录后显示登录方式、设备、四个功能开关、退出登录与删除账号。
// 开关改的是服务器上的同意记录，成功后桥同时写回 cloud.toml，键盘下次弹出时按新开关连。

import SwiftUI

struct AccountView: View {
    let store: AccountStore

    @State private var confirmingSignOut = false

    @State private var confirmingDelete = false

    var body: some View {
        Form {
            if let message = store.message {
                Section { Text(message).foregroundStyle(.secondary) }
            }
            if let state = store.state {
                if state.signedIn {
                    signedIn(state)
                } else {
                    SignInSections(store: store)
                }
                Section {
                    LabeledContent("服务器", value: state.server)
                } footer: {
                    Text("没登录时键盘完全离线。登录后的功能要在系统设置里给青简打开「允许完全访问」才能联网。")
                }
            } else {
                ProgressView()
            }
        }
        .navigationTitle("账号")
        .disabled(store.busy)
        .overlay {
            if store.busy { ProgressView() }
        }
        .task { await store.refresh() }
    }

    @ViewBuilder
    private func signedIn(_ state: AccountState) -> some View {
        Section("登录方式") {
            ForEach(state.identities, id: \.self) { identity in
                LabeledContent(identity.title, value: identity.label ?? "")
            }
        }
        Section {
            ForEach(CloudFeature.allCases) { feature in
                Toggle(feature.title, isOn: Binding(
                    get: { store.state?.consents[feature] ?? false },
                    set: { value in Task { await store.setConsent(feature, value) } }))
            }
        } header: {
            Text("功能")
        } footer: {
            Text("都默认关闭。关掉某项会同时删除服务器上这部分数据，本机数据不受影响。")
        }
        Section("设备") {
            ForEach(state.sessions) { device in
                DeviceRow(device: device) { Task { await store.revoke(device) } }
            }
        }
        Section {
            Button("退出登录") { confirmingSignOut = true }
            Button("删除账号", role: .destructive) { confirmingDelete = true }
        }
        .confirmationDialog("退出登录？", isPresented: $confirmingSignOut, titleVisibility: .visible) {
            Button("退出登录", role: .destructive) { Task { await store.signOut() } }
        } message: {
            Text("退出后键盘不再连服务器，本机的输入习惯保留。")
        }
        .confirmationDialog("删除账号？", isPresented: $confirmingDelete, titleVisibility: .visible) {
            Button("永久删除账号", role: .destructive) { Task { await store.deleteAccount() } }
        } message: {
            Text("会立即删除服务器上这个账号的全部数据（剪贴板、学习数据、设置、输入日志）、撤销 Apple 授权并让所有设备退出登录，无法恢复。本机的输入习惯不受影响。")
        }
    }
}
```

`App/SetupView.swift` 整个文件替换为：

```swift
// 主 App 首页：启用步骤、设置与账号入口、试打框。

import SwiftUI
import UIKit

struct SetupView: View {
    @State private var draft = ""

    @State private var store = SettingsStore()

    @State private var account = AccountStore()

    var body: some View {
        NavigationStack {
            Form {
                Section("启用键盘") {
                    Label("打开「设置 → 通用 → 键盘 → 键盘」", systemImage: "1.circle")
                    Label("点「添加新键盘…」，选「青简」", systemImage: "2.circle")
                    Label("打字时长按地球键切到青简", systemImage: "3.circle")
                    Button("打开设置") {
                        if let url = URL(string: UIApplication.openSettingsURLString) {
                            UIApplication.shared.open(url)
                        }
                    }
                }
                Section {
                    if store.available {
                        NavigationLink("键盘设置") { KeyboardSettingsView(store: store) }
                        NavigationLink("账号") { AccountView(store: account) }
                    } else {
                        Text("这个安装包没有开通 App Group，设置改不到键盘上。").foregroundStyle(.secondary)
                    }
                } header: {
                    Text("设置")
                } footer: {
                    Text("与 Mac 版偏好设置是同一份，登录并打开同步后两边互通。")
                }
                Section {
                    TextField("在这里试打", text: $draft, axis: .vertical)
                        .lineLimit(3...8)
                } header: {
                    Text("试一试")
                } footer: {
                    Text("「完全访问」用于按键震动，以及登录后连接服务器（大模型润色、剪贴板与学习数据同步）。不开也能正常打字；没登录时键盘不联网。")
                }
            }
            .navigationTitle("青简")
        }
    }
}
```

- [ ] **Step 8: 模拟器编译通过**

Run:
```bash
cd /Users/liyuqing/sproot/qingjian-mainline/cloud/ios
xcodegen generate
xcodebuild -scheme QingjianCloud -destination 'platform=iOS Simulator,name=iPhone 17e' build 2>&1 | tail -3
grep -c '^token' Keyboard/Data/cloud.toml || true
```
Expected: `** BUILD SUCCEEDED **`；最后一行输出 `0`（随包种子里没有令牌）。若 Swift 6 报 `ASAuthorization` 跨隔离的 Sendable 错误，把 `SignInSections` 里的 `Task { await store.completeApple(result) }` 改成 `Task { @MainActor in await store.completeApple(result) }` 再编。

模拟器里打开 App → 账号：显示「登录」「用邮箱登录」两段与「服务器 https://pinyin.synon.ai」；不需要服务端就能看到这一屏（`qj_account_status` 没登录时不联网）。

- [ ] **Step 9: 真机验证（Apple 登录只能真机；要等服务端部署）**

前提：服务端已部署，`SUJIAN_APPLE_IOS_CLIENT_ID=app.qingjian.cloud`、SMTP 已配；Xcode → Settings → Accounts 登录团队 L9YRXEKYN2（自动签名会给 App ID `app.qingjian.cloud` 加上 Sign in with Apple）。

1. `cd /Users/liyuqing/sproot/qingjian-mainline/cloud/ios && scripts/install-device.sh`，装好后 App 启动。
2. **旧安装升级**：如果这台手机以前装过带 `qjc_` 令牌的版本，账号页显示未登录；关 Wi-Fi 在备忘录里打字，候选、学习照常，候选栏没有云端词、没有剪贴板提示。
3. **Apple 登录**：账号 → 通过 Apple 登录 → Face ID。回到账号页：登录方式「Apple」，设备列表里这台 iPhone 标「本机」，四个开关全关。服务器上 `sujian-admin user list` 多了一个用户、身份 apple、会话 1。
4. 打开「同步学习数据与设置」：开关保持打开；服务器上该用户的 consents 里 sync=1。切到备忘录弹出青简键盘打几个字，半分钟后服务器有这个用户的 learning 行。
5. 开飞行模式，点开「跨设备剪贴板」：开关弹回关，页面顶上显示「连不上服务器，检查网络后再试」。关飞行模式再开，成功。
6. **邮箱登录**：退出登录（确认）→ 页面回到未登录、四个开关消失；用邮箱登录：输邮箱 → 发送验证码 → 邮件里 6 位码 → 登录。先故意输错一次，显示「验证码不对或已过期」。
7. 在 Mac 上用同一个邮箱登录（Task 5 完成后），iPhone 账号页的设备列表出现那台 Mac，点「注销」→ 确认，Mac 的菜单变成未登录。
8. **删账号**（用测试账号）：删除账号 → 「永久删除账号」。页面回到未登录，服务器 `sujian-admin user list` 里没有这个用户。
9. 全程关 Wi-Fi 打字不受影响、没有弹窗。

- [ ] **Step 10: 提交**

```bash
cd /Users/liyuqing/sproot/qingjian-mainline
git add cloud/ios/project.yml cloud/ios/App cloud/ios/scripts/build-bridge.sh cloud/ios/scripts/install-device.sh cloud/ios/cloud.example.toml
git status --short cloud/ios   # 确认 Keyboard/Sources 的无关改动没被暂存
git commit -m "$(cat <<'EOF'
feat(cloud): iOS「青简 Cloud」页改成账号页，支持 Apple 与邮箱登录

未登录时 Sign in with Apple（nonce 的 SHA-256 给 Apple、原始值给桥）与邮箱验证码两种；登录后显示登录方式、设备（可注销）、
四个功能开关（失败回滚）、退出登录与删除账号（二次确认）。App 加 Sign in with Apple 权限。
构建只往包里写 server，不再拷 cloud.local.toml 的令牌；install-device.sh 发现包里带令牌就拒装。键盘不改，没登录完全离线。

EOF
)"
```

---

## Task 5: Mac（qingjian-cloud-mac）的网页登录与开关

**Files:**
- Modify: `cloud/crates/qingjian-cloud-mac/Cargo.toml`、`src/lib.rs`、`src/config.rs`、`src/menu/display.rs`、`README.md`
- Create: `src/account/{mod,event,anchor,web_login}.rs`、`src/menu/account_menu.rs`、`src/service/account.rs`
- Move: `src/service.rs` → `src/service/mod.rs`（并改）
- Rewrite: `src/menu/mod.rs`
- Modify: 仓库根 `Cargo.lock`、`cloud/docs/fork-patch.md`
- Test: `src/account/mod.rs`、`src/config.rs`、`src/menu/mod.rs` 内的 `#[cfg(test)] mod tests`

现状：`config.toml` 里 `server` 与 `token`（`qjc_`）手填，`AgentConfig::load` 没填完就返回 `Err("未配置…")`；开关是 `learning` / `settings` / `logs` / `download_logs`，缺省全开，没有剪贴板与大模型开关（大模型端点只要配好就给）。菜单只有 `Text` / `Action` / `Separator` 三种行，画法在输入法侧的 `apps/macos/src/menubar/cloud_agent.rs`（分叉文件，本任务不改它），所以开关画成「跨设备剪贴板：开」这样的可点项。
**IME 里有没有可用窗口**：输入法是 `LSBackgroundOnly`、激活策略平时 `Prohibited`，没有常驻窗口；偏好设置窗口（`apps/macos/src/preferences/panel.rs`）的做法是临时切 `Accessory`、`activateIgnoringOtherApps`、`makeKeyAndOrderFront`，关窗切回 `Prohibited`。登录照这个做：建一个无边框、透明（alpha 0）、居中的 1×1 窗口当 `ASPresentationAnchor`，由实现 `ASWebAuthenticationPresentationContextProviding` 的 `LoginAnchor`（objc2 `define_class!`）返回；`presentationContextProvider` 是弱引用，`WebLogin` 自己握着 `LoginAnchor`。登录窗关掉（`WebLogin` 被丢弃）时取消会话、关锚点窗口、切回 `Prohibited`。
`initWithURL:callback:completionHandler:` 要 macOS 14.4，输入法支持到 13.0，所以用已标弃用的 `initWithURL:callbackURLScheme:completionHandler:`（`#[allow(deprecated)]`）。

需要的依赖与 feature：
- 跨平台：`base64 0.22`（URL_SAFE_NO_PAD）、`getrandom 0.3`（`fill`）、`percent-encoding 2`、`sha2 0.11`（根 Cargo.lock 已有 0.11.0）、`toml_edit 0.25`（与 bridge 同版本）。
- macOS：`objc2-authentication-services 0.3.2`（关默认 feature，开 `std`、`block2`、`ASFoundation`、`ASWebAuthenticationSession`）、`block2 0.6.2`；`objc2-foundation` 加 `NSError`、`NSGeometry`、`NSURL`；`objc2-app-kit` 加 `NSApplication`、`NSGraphics`、`NSResponder`、`NSRunningApplication`、`NSWindow`。

- [ ] **Step 1: 加依赖**

```bash
cd /Users/liyuqing/sproot/qingjian-mainline/cloud
cargo add -p qingjian-cloud-mac base64@0.22 getrandom@0.3 percent-encoding@2 sha2@0.11 toml_edit@0.25
cargo add -p qingjian-cloud-mac --target 'cfg(target_os = "macos")' block2@0.6.2
cargo add -p qingjian-cloud-mac --target 'cfg(target_os = "macos")' objc2-authentication-services@0.3.2 --no-default-features -F std,block2,ASFoundation,ASWebAuthenticationSession
cargo add -p qingjian-cloud-mac --target 'cfg(target_os = "macos")' objc2-foundation@0.3.2 -F NSError,NSGeometry,NSURL
cargo add -p qingjian-cloud-mac --target 'cfg(target_os = "macos")' objc2-app-kit@0.3.2 -F NSApplication,NSGraphics,NSResponder,NSRunningApplication,NSWindow
```

完成后 `Cargo.toml` 的依赖段应为（`cargo add` 排序可能略有不同，内容一致即可）：

```toml
[dependencies]
base64 = "0.22"
getrandom = "0.3"
percent-encoding = "2"
qingjian-cloud-client = { workspace = true, features = ["native-tls"] }
qingjian-cloud-proto.workspace = true
serde.workspace = true
sha2 = "0.11"
toml.workspace = true
toml_edit = "0.25"
tracing.workspace = true

[target.'cfg(target_os = "macos")'.dependencies]
block2 = "0.6.2"
objc2 = "0.6.4"
objc2-app-kit = { version = "0.3.2", features = ["NSApplication", "NSGraphics", "NSPasteboard", "NSResponder", "NSRunningApplication", "NSWindow"] }
objc2-authentication-services = { version = "0.3.2", default-features = false, features = ["std", "block2", "ASFoundation", "ASWebAuthenticationSession"] }
objc2-foundation = { version = "0.3.2", features = ["NSDate", "NSError", "NSGeometry", "NSObject", "NSRunLoop", "NSString", "NSTimer", "NSURL"] }
```

Run: `cargo check -p qingjian-cloud-mac` → Expected: 通过（只加了依赖）。

- [ ] **Step 2: 写失败的测试（verifier / challenge / 登录地址 / 回调解析 / 设备名）**

在 `src/lib.rs` 的 `mod config;` 前加一行 `mod account;`，新建 `src/account/mod.rs`，**先只放测试**：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 7636 附录 B 的例子。
    const RFC_VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";

    #[test]
    fn challenge_matches_rfc_7636() {
        assert_eq!(
            challenge(RFC_VERIFIER),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn verifier_is_43_chars_of_base64url() {
        let verifier = verifier_from(&[0xff; 32]);
        assert_eq!(verifier.len(), 43);
        assert!(verifier.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'));
        let random = new_verifier().unwrap();
        assert_eq!(random.len(), 43);
        assert_ne!(random, new_verifier().unwrap());
    }

    #[test]
    fn login_url_escapes_device_name() {
        let url = login_url("https://pinyin.synon.ai/", "abc-_", "李的 MacBook");
        assert_eq!(
            url,
            "https://pinyin.synon.ai/login?challenge=abc-_&device=%E6%9D%8E%E7%9A%84%20MacBook&callback=sujian"
        );
    }

    #[test]
    fn handoff_comes_from_the_sujian_callback_only() {
        assert_eq!(
            handoff_from_callback("sujian://auth?handoff=h%2B1&x=y").as_deref(),
            Some("h+1")
        );
        assert_eq!(
            handoff_from_callback("SUJIAN://auth/?x=1&handoff=abc#frag").as_deref(),
            Some("abc")
        );
        assert_eq!(handoff_from_callback("https://auth?handoff=abc"), None);
        assert_eq!(handoff_from_callback("sujian://other?handoff=abc"), None);
        assert_eq!(handoff_from_callback("sujian://auth?handoff="), None);
        assert_eq!(handoff_from_callback("sujian://auth"), None);
    }

    #[test]
    fn device_name_falls_back_and_is_trimmed() {
        assert_eq!(clean_device_name(None), "Mac");
        assert_eq!(clean_device_name(Some("  \n".to_owned())), "Mac");
        assert_eq!(clean_device_name(Some("李的 MacBook\n".to_owned())), "李的 MacBook");
        assert_eq!(clean_device_name(Some("长".repeat(100))).chars().count(), 64);
    }
}
```

- [ ] **Step 3: 跑测试，确认失败**

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud && cargo test -p qingjian-cloud-mac account::tests`
Expected: 编译失败，`cannot find function challenge in this scope`（`verifier_from`、`new_verifier`、`login_url`、`handoff_from_callback`、`clean_device_name` 同）。

- [ ] **Step 4: 实现 `account` 模块**

`src/account/mod.rs` 整个文件写成（Step 2 的测试模块原样留在末尾）：

```rust
//! 账号的可移植部分：PKCE 的 verifier / challenge、网页登录页地址、回跳地址里的一次性码、设备名、给用户看的错误。
//! 真正打开登录窗口的 [`WebLogin`] 与它的展示锚点只在 macOS 上编。

mod event;

#[cfg(target_os = "macos")]
mod anchor;
#[cfg(target_os = "macos")]
mod web_login;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, percent_decode_str, utf8_percent_encode};
use qingjian_cloud_client::ClientError;
use qingjian_cloud_proto::{LOGIN_CALLBACK_SCHEME, PATH_LOGIN};
use sha2::{Digest, Sha256};

pub use self::event::AccountEvent;

#[cfg(target_os = "macos")]
pub use self::anchor::LoginAnchor;
#[cfg(target_os = "macos")]
pub use self::web_login::WebLogin;

/// 查询参数里要转义的字符：RFC 3986 的非保留字符之外都转。
const QUERY: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

/// 回跳地址的主机部分：`sujian://auth?handoff=…`。
const CALLBACK_HOST: &str = "auth";

/// 设备名取不到时报给服务端的名字。
const FALLBACK_DEVICE: &str = "Mac";

/// 设备名最长多少个字符。
const MAX_DEVICE_CHARS: usize = 64;

/// 32 字节随机数的 base64url（无填充，43 个字符），只留在本进程里。
pub fn new_verifier() -> Result<String, String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|error| format!("取不到随机数：{error}"))?;
    Ok(verifier_from(&bytes))
}

pub fn verifier_from(bytes: &[u8; 32]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

/// PKCE 的 S256：`base64url(SHA256(verifier))`，无填充。
pub fn challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

/// `{server}/login?challenge=…&device=…&callback=sujian`。
pub fn login_url(server: &str, challenge: &str, device: &str) -> String {
    format!(
        "{}{PATH_LOGIN}?challenge={}&device={}&callback={LOGIN_CALLBACK_SCHEME}",
        server.trim().trim_end_matches('/'),
        utf8_percent_encode(challenge, QUERY),
        utf8_percent_encode(device, QUERY),
    )
}

/// 从 `sujian://auth?handoff=…` 取出一次性码；别的地址、没有或为空返回 `None`。
pub fn handoff_from_callback(url: &str) -> Option<String> {
    let (scheme, rest) = url.split_once("://")?;
    if !scheme.eq_ignore_ascii_case(LOGIN_CALLBACK_SCHEME) {
        return None;
    }
    let (host, query) = rest.split_once('?')?;
    if !host.trim_end_matches('/').eq_ignore_ascii_case(CALLBACK_HOST) {
        return None;
    }
    let query = query.split('#').next().unwrap_or_default();
    query
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .find(|(key, _)| *key == "handoff")
        .and_then(|(_, value)| percent_decode_str(value).decode_utf8().ok())
        .map(|value| value.into_owned())
        .filter(|value| !value.is_empty())
}

/// 「系统设置 → 通用 → 关于本机」里的电脑名，设备列表里显示。
pub fn device_name() -> String {
    let output = std::process::Command::new("scutil")
        .args(["--get", "ComputerName"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok());
    clean_device_name(output)
}

fn clean_device_name(raw: Option<String>) -> String {
    raw.map(|name| name.trim().chars().take(MAX_DEVICE_CHARS).collect::<String>())
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| FALLBACK_DEVICE.to_owned())
}

/// 给用户看的失败原因，菜单里显示。
pub fn reason(error: &ClientError) -> String {
    match error {
        ClientError::Unreachable(_) | ClientError::Io(_) => "连不上服务器".to_owned(),
        ClientError::Unauthorized => "登录没有通过或已失效".to_owned(),
        ClientError::Forbidden(_) => "服务器不允许".to_owned(),
        ClientError::RateLimited => "操作太频繁，稍后再试".to_owned(),
        ClientError::Rejected { status, .. } => format!("服务器拒绝了请求（{status}）"),
        ClientError::BadResponse(_) => "服务器的回应看不懂".to_owned(),
    }
}
```

`src/account/event.rs`：

```rust
//! 账号操作的结果：登录窗口的回调与后台线程（换令牌、切开关、取账号）经通道交给主线程拍子。

use qingjian_cloud_proto::{Consents, Feature};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountEvent {
    /// 网页登录窗口结束：回跳的地址，或失败 / 取消的原因。
    Callback(Result<String, String>),

    /// 用一次性码换到了令牌，以及服务器上的开关（取不到为全关）。
    SignedIn { token: String, consents: Consents },

    /// 服务器上的开关（切换成功或刷新得到）。
    Consents(Consents),

    /// 切换开关时服务器说这项不能开（403）：显示为关。
    Forbidden(Feature),

    /// 令牌失效（401）：清掉令牌，回到未登录。
    SignedOut,

    /// 给用户看的失败原因，显示在菜单里。
    Failed(String),
}
```

`src/account/anchor.rs`：

```rust
//! `ASWebAuthenticationSession` 要的展示锚点。输入法是 LSBackgroundOnly 进程，平时没有窗口：
//! 登录时临时建一个无边框、透明的 1×1 窗口放在主屏中央当锚点，登录结束由 `WebLogin` 关掉。
//! 会话的 presentationContextProvider 是弱引用，`WebLogin` 要自己握着本对象。

use objc2::rc::Retained;
use objc2::runtime::NSObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSBackingStoreType, NSWindow, NSWindowStyleMask};
use objc2_authentication_services::{
    ASPresentationAnchor, ASWebAuthenticationPresentationContextProviding,
    ASWebAuthenticationSession,
};
use objc2_foundation::{NSObjectProtocol, NSPoint, NSRect, NSSize};

define_class!(
    // SAFETY: NSObject 允许子类化；没有实现 Drop。
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Retained<NSWindow>]
    pub struct LoginAnchor;

    unsafe impl NSObjectProtocol for LoginAnchor {}

    unsafe impl ASWebAuthenticationPresentationContextProviding for LoginAnchor {
        #[unsafe(method_id(presentationAnchorForWebAuthenticationSession:))]
        fn presentation_anchor(
            &self,
            _session: &ASWebAuthenticationSession,
        ) -> Retained<ASPresentationAnchor> {
            // ASPresentationAnchor 在绑定里是 NSObject：NSWindow → NSResponder → NSObject
            Retained::into_super(Retained::into_super(self.ivars().clone()))
        }
    }
);

impl LoginAnchor {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1.0, 1.0));
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                mtm.alloc::<NSWindow>(),
                frame,
                NSWindowStyleMask::Borderless,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        // 程序建的 NSWindow 默认关窗即释放，我们还握着 Retained，必须关掉
        unsafe { window.setReleasedWhenClosed(false) };
        window.setAlphaValue(0.0);
        window.center();
        window.orderFrontRegardless();
        let this = mtm.alloc::<Self>().set_ivars(window);
        unsafe { msg_send![super(this), init] }
    }

    pub fn close(&self) {
        self.ivars().close();
    }
}
```

`src/account/web_login.rs`：

```rust
//! 网页登录：用 `ASWebAuthenticationSession` 打开服务端的登录页（Apple 与邮箱都在页面上），
//! 等它回跳 `sujian://auth?handoff=…`。结果经通道交给主线程拍子；丢掉本对象就取消登录、关掉锚点窗口。

use std::sync::mpsc::Sender;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{AnyThread, MainThreadMarker};
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
use objc2_authentication_services::{
    ASWebAuthenticationSession, ASWebAuthenticationSessionErrorCode,
};
use objc2_foundation::{NSError, NSString, NSURL};
use qingjian_cloud_proto::LOGIN_CALLBACK_SCHEME;

use super::{AccountEvent, LoginAnchor};

pub struct WebLogin {
    session: Retained<ASWebAuthenticationSession>,

    anchor: Retained<LoginAnchor>,

    /// PKCE 的 verifier：只在本进程里，换令牌时交给服务端核对。
    verifier: String,

    /// 报给服务端的设备名。
    device: String,
}

impl WebLogin {
    /// 打开登录窗口；`url` 是 [`super::login_url`] 拼好的地址。
    pub fn start(
        mtm: MainThreadMarker,
        url: &str,
        verifier: String,
        device: String,
        sender: Sender<AccountEvent>,
    ) -> Result<Self, String> {
        let url = NSURL::URLWithString(&NSString::from_str(url))
            .ok_or_else(|| "登录地址无效".to_owned())?;
        let handler: RcBlock<dyn Fn(*mut NSURL, *mut NSError)> =
            RcBlock::new(move |callback: *mut NSURL, error: *mut NSError| {
                // SAFETY: 系统给的指针要么为空要么在回调期间有效
                let (callback, error) = unsafe { (callback.as_ref(), error.as_ref()) };
                let result = match (callback, error) {
                    (Some(callback), _) => callback
                        .absoluteString()
                        .map(|text| text.to_string())
                        .ok_or_else(|| "登录回调没有地址".to_owned()),
                    (None, Some(error))
                        if error.code() == ASWebAuthenticationSessionErrorCode::CanceledLogin.0 =>
                    {
                        Err("已取消登录".to_owned())
                    }
                    (None, Some(error)) => {
                        Err(format!("登录窗口出错：{}", error.localizedDescription()))
                    }
                    (None, None) => Err("登录没有完成".to_owned()),
                };
                let _ = sender.send(AccountEvent::Callback(result));
            });
        let scheme = NSString::from_str(LOGIN_CALLBACK_SCHEME);
        // initWithURL:callback:completionHandler: 要 macOS 14.4，输入法支持到 13
        #[allow(deprecated)]
        let session = unsafe {
            ASWebAuthenticationSession::initWithURL_callbackURLScheme_completionHandler(
                ASWebAuthenticationSession::alloc(),
                &url,
                Some(&scheme),
                RcBlock::as_ptr(&handler),
            )
        };
        let anchor = LoginAnchor::new(mtm);
        unsafe { session.setPresentationContextProvider(Some(ProtocolObject::from_ref(&*anchor))) };
        // 照偏好设置窗口的做法：切到 Accessory 并激活，登录窗才拿得到键盘焦点
        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
        #[allow(deprecated)]
        app.activateIgnoringOtherApps(true);
        if !unsafe { session.start() } {
            anchor.close();
            app.setActivationPolicy(NSApplicationActivationPolicy::Prohibited);
            return Err("登录窗口打不开".to_owned());
        }
        Ok(Self {
            session,
            anchor,
            verifier,
            device,
        })
    }

    pub fn verifier(&self) -> &str {
        &self.verifier
    }

    pub fn device(&self) -> &str {
        &self.device
    }
}

impl Drop for WebLogin {
    /// 已经结束的会话 cancel 是空操作；关锚点窗口，输入法回到纯后台。
    fn drop(&mut self) {
        unsafe { self.session.cancel() };
        self.anchor.close();
        let mtm = MainThreadMarker::from(&*self.anchor);
        NSApplication::sharedApplication(mtm)
            .setActivationPolicy(NSApplicationActivationPolicy::Prohibited);
    }
}
```

- [ ] **Step 5: 跑测试，确认通过**

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud && cargo test -p qingjian-cloud-mac account::tests && cargo check -p qingjian-cloud-mac`
Expected: 5 个测试 PASS；`cargo check` 通过（`WebLogin` / `LoginAnchor` 还没被用，`#![cfg_attr(not(target_os = "macos"), allow(dead_code))]` 只管非 macOS——macOS 上若报 dead_code 警告，到 Step 10 接上 `Service` 后消失，这一步不提交）。

- [ ] **Step 6: 写失败的测试（config.toml）**

`src/config.rs` 末尾的测试模块整个替换为：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("qjc-mac-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("config.toml")
    }

    #[test]
    fn missing_file_writes_template_that_reads_as_signed_out() {
        let path = temp("template");
        let config = AgentConfig::load(&path).unwrap();
        assert!(!config.signed_in());
        assert_eq!(config.server(), DEFAULT_SERVER);
        assert_eq!(config.consents(), Consents::default());
        assert!(config.download_logs);
        assert!(std::fs::read_to_string(&path).unwrap().contains("token = \"\""));
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn old_device_token_is_signed_out() {
        let path = temp("old");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "server = \"https://x/\"\ntoken = \"qjc_1\"\nlearning = true\n").unwrap();
        let config = AgentConfig::load(&path).unwrap();
        assert!(!config.signed_in());
        assert_eq!(config.server(), "https://x");
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn store_session_keeps_comments_and_sets_switches() {
        let path = temp("session");
        AgentConfig::load(&path).unwrap();
        let consents = Consents {
            sync: true,
            llm: true,
            ..Consents::default()
        };
        AgentConfig::store_session(&path, "sjt_x", consents).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        assert!(text.contains("# 青简 Cloud 配置"));
        let config = AgentConfig::load(&path).unwrap();
        assert!(config.signed_in());
        assert_eq!(config.token, "sjt_x");
        assert_eq!(config.consents(), consents);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }

    #[test]
    fn store_consents_and_clear_session() {
        let path = temp("clear");
        AgentConfig::store_session(&path, "sjt_x", Consents::default()).unwrap();
        let consents = Consents {
            clipboard: true,
            input_log: true,
            ..Consents::default()
        };
        AgentConfig::store_consents(&path, consents).unwrap();
        let config = AgentConfig::load(&path).unwrap();
        assert_eq!(config.token, "sjt_x");
        assert!(config.clipboard && config.logs && !config.sync);
        AgentConfig::clear_session(&path).unwrap();
        let config = AgentConfig::load(&path).unwrap();
        assert!(!config.signed_in());
        assert_eq!(config.consents(), Consents::default());
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
```

Run: `cargo test -p qingjian-cloud-mac config::tests`
Expected: 编译失败，`cannot find value DEFAULT_SERVER`、`no method named signed_in`、`no function store_session` 等。

- [ ] **Step 7: 重写 `config.rs`（测试模块保留在末尾）**

```rust
//! `QingjianCloud/config.toml`：服务器地址、登录得到的会话令牌与四个功能开关。第一次运行写出一份带注释的模板。
//! 令牌由菜单里的「登录…」写入、「退出登录」清空；开关在菜单里切换（先告诉服务器）。都经 `toml_edit` 改，注释保留。
//! 不并进输入法的 `config.toml`：那份会同步到别的设备，令牌是每台设备自己的。

use std::path::Path;

use qingjian_cloud_proto::{Consents, TOKEN_PREFIX};
use serde::Deserialize;
use toml_edit::{DocumentMut, value};

/// 模板里的服务器，也是配置里没写地址时用的。
pub const DEFAULT_SERVER: &str = "https://pinyin.synon.ai";

/// 第一次运行写出的模板。令牌只在本机，文件权限设成仅本人可读。
const TEMPLATE: &str = r#"# 青简 Cloud 配置。登录与功能开关都在输入法「中☁ → 青简 Cloud ›」里操作，一般不用手改。
# 服务器地址，带 https://
server = "https://pinyin.synon.ai"

# 会话令牌：由菜单里的「登录…」写入，「退出登录」清空。不要手填，也不要发给别人
token = ""

# 以下四项与服务器上的同意记录一致，在菜单里切换（会先告诉服务器）；手改不会同步到服务器
# 跨设备剪贴板
clipboard = false

# 同步输入法的学习数据与 config.toml（设置与自定义短语）
sync = false

# 上传输入法的输入日志（input-log.jsonl）
logs = false

# 云联想走服务器的大模型
llm = false

# 开了 logs 时，把别的设备的输入日志下载到 ~/Library/Application Support/QingjianCloud/input-log/
download_logs = true
"#;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct AgentConfig {
    pub server: String,

    /// 会话令牌（`sjt_` 开头），由登录写入。旧版手填的设备令牌（`qjc_`）已作废，当没登录。
    pub token: String,

    /// 跨设备剪贴板（服务器上的 `clipboard`）。
    pub clipboard: bool,

    /// 同步学习数据与 `config.toml`（服务器上的 `sync`）。
    pub sync: bool,

    /// 上传输入日志（服务器上的 `input_log`）。
    pub logs: bool,

    /// 云联想走服务器的大模型（服务器上的 `llm`）。
    pub llm: bool,

    /// 开了 `logs` 时下载别的设备的输入日志；只在本机，不经服务器。
    pub download_logs: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            server: String::new(),
            token: String::new(),
            clipboard: false,
            sync: false,
            logs: false,
            llm: false,
            download_logs: true,
        }
    }
}

impl AgentConfig {
    /// 读配置；文件不存在就写模板并按模板读。读不了或格式不对返回 `Err`，带给用户看的原因。没登录不算错。
    pub fn load(path: &Path) -> Result<Self, String> {
        let text = match std::fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                if let Err(reason) = write_private(path, TEMPLATE) {
                    tracing::warn!(%reason, "配置模板写入失败");
                }
                TEMPLATE.to_owned()
            }
            Err(error) => return Err(format!("配置文件读不了：{error}")),
        };
        toml::from_str(&text).map_err(|error| format!("配置文件有错：{error}"))
    }

    pub fn signed_in(&self) -> bool {
        self.token.trim().starts_with(TOKEN_PREFIX)
    }

    /// 去掉末尾的 `/`；没写地址时用 [`DEFAULT_SERVER`]。
    pub fn server(&self) -> String {
        let server = self.server.trim().trim_end_matches('/');
        if server.is_empty() {
            DEFAULT_SERVER.to_owned()
        } else {
            server.to_owned()
        }
    }

    pub fn consents(&self) -> Consents {
        Consents {
            clipboard: self.clipboard,
            sync: self.sync,
            input_log: self.logs,
            llm: self.llm,
        }
    }

    /// 登录成功：写入令牌与服务器上的开关。
    pub fn store_session(path: &Path, token: &str, consents: Consents) -> Result<(), String> {
        edit(path, |document| {
            document["token"] = value(token);
            set_consents(document, consents);
        })
    }

    /// 服务器上的开关变了：只改开关。
    pub fn store_consents(path: &Path, consents: Consents) -> Result<(), String> {
        edit(path, |document| set_consents(document, consents))
    }

    /// 退出登录、令牌失效：清掉令牌与开关。
    pub fn clear_session(path: &Path) -> Result<(), String> {
        edit(path, |document| {
            document["token"] = value("");
            set_consents(document, Consents::default());
        })
    }
}

fn set_consents(document: &mut DocumentMut, consents: Consents) {
    document["clipboard"] = value(consents.clipboard);
    document["sync"] = value(consents.sync);
    document["logs"] = value(consents.input_log);
    document["llm"] = value(consents.llm);
}

/// 读出文件（没有就从模板开始）、改、写回，注释与别的键原样保留。
fn edit(path: &Path, change: impl FnOnce(&mut DocumentMut)) -> Result<(), String> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => TEMPLATE.to_owned(),
        Err(error) => return Err(format!("配置文件读不了：{error}")),
    };
    let mut document: DocumentMut = text
        .parse()
        .map_err(|error| format!("配置文件有错：{error}"))?;
    change(&mut document);
    write_private(path, &document.to_string())
}

/// 写文件并设成仅本人可读写（里面有令牌）。
fn write_private(path: &Path, text: &str) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|error| error.to_string())?;
    }
    std::fs::write(path, text).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}
```

Run: `cargo test -p qingjian-cloud-mac config::tests`
Expected: 4 个测试 PASS（`service.rs` 用了旧字段，**这时整个 crate 在 macOS 上编不过**——`cargo test` 会因 `service.rs` 报 `no field learning` 失败；这种情况下先做 Step 8–10 再一起跑。Linux 上 `service` 不参与编译，可以单独跑通）。

- [ ] **Step 8: 写失败的测试（菜单），重写菜单**

`src/menu/display.rs` 整个文件替换为：

```rust
//! 「青简 Cloud ›」子菜单第一行显示的状态。

use qingjian_cloud_client::Status;

/// 子菜单显示的状态：连接状态之外还有「没配置好」「没登录」「登录了但没开剪贴板」与「暂停」。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Display {
    /// 配置文件读不了或同步起不来，带原因。
    Unconfigured(String),

    /// 还没登录（或令牌失效后清掉了）。
    SignedOut,

    /// 登录了，但没开跨设备剪贴板，没有连接状态可显示。
    SignedIn,

    Paused,

    Sync { status: Status, pending: usize },
}
```

`src/menu/account_menu.rs`：

```rust
//! 菜单里账号那几行要的状态：登没登录、是不是正在登录、四个开关、最近一次失败的原因。

use qingjian_cloud_proto::Consents;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountMenu {
    pub signed_in: bool,

    /// 网页登录窗口开着，或正在用一次性码换令牌。
    pub signing_in: bool,

    pub consents: Consents,

    /// 最近一次登录或切换开关失败的原因。
    pub note: Option<String>,
}
```

`src/menu/mod.rs` 整个文件替换为：

```rust
//! 菜单内容：连接状态、账号（登录 / 四个开关 / 退出登录）、学习数据状态、最近的剪贴板（点一条复制到本机）、暂停与几个操作。
//! 只算出行列表，由输入法画成「中☁ → 青简 Cloud ›」子菜单（输入法只认文字行与可点项，开关画成「名字：开 / 关」）。
//! tag 区分动作：非负数是历史条目的下标，负数是固定动作（输入法转发 -100..100 的 tag）。

mod account_menu;
mod display;
mod lines;

use qingjian_cloud_client::{DataStatus, Status};
use qingjian_cloud_proto::Feature;

use crate::history::History;

pub use account_menu::AccountMenu;
pub use display::Display;
pub use lines::Line;

pub const TAG_PAUSE: isize = -1;
pub const TAG_RELOAD: isize = -2;
pub const TAG_OPEN_CONFIG: isize = -3;
pub const TAG_SYNC_NOW: isize = -6;
pub const TAG_SIGN_IN: isize = -7;
pub const TAG_SIGN_OUT: isize = -8;
pub const TAG_CANCEL_SIGN_IN: isize = -9;

/// 四个功能开关的 tag：-10 起按 [`Feature::ALL`] 的顺序往下排（-10..=-13）。
const TAG_TOGGLE_FIRST: isize = -10;

pub fn toggle_tag(feature: Feature) -> isize {
    let index = Feature::ALL
        .iter()
        .position(|&f| f == feature)
        .unwrap_or_default();
    TAG_TOGGLE_FIRST - index as isize
}

pub fn toggled_feature(tag: isize) -> Option<Feature> {
    let index = usize::try_from(TAG_TOGGLE_FIRST - tag).ok()?;
    Feature::ALL.get(index).copied()
}

pub fn build_lines(
    display: &Display,
    account: &AccountMenu,
    data: Option<&DataStatus>,
    history: &History,
) -> Vec<Line> {
    let mut lines = vec![Line::Text(status_line(display))];
    if let Some(data) = data {
        lines.push(Line::Text(data_line(data)));
    }
    if let Some(note) = &account.note {
        lines.push(Line::Text(short(note)));
    }
    lines.push(Line::Separator);
    if account.signed_in {
        for feature in Feature::ALL {
            let state = if account.consents.get(feature) { "开" } else { "关" };
            lines.push(Line::Action(
                format!("{}：{state}", feature_title(feature)),
                toggle_tag(feature),
            ));
        }
        lines.push(Line::Separator);
        if account.consents.clipboard {
            let before = lines.len();
            for (index, entry) in history.entries().enumerate() {
                lines.push(Line::Action(entry.title(), index as isize));
            }
            if lines.len() == before {
                lines.push(Line::Text("还没有剪贴板记录".to_owned()));
            }
            lines.push(Line::Separator);
        }
        let pause = if matches!(display, Display::Paused) {
            "继续同步"
        } else {
            "暂停同步"
        };
        lines.push(Line::Action(pause.to_owned(), TAG_PAUSE));
        if data.is_some() {
            lines.push(Line::Action("立即同步学习数据".to_owned(), TAG_SYNC_NOW));
        }
        lines.push(Line::Action("退出登录".to_owned(), TAG_SIGN_OUT));
    } else if account.signing_in {
        lines.push(Line::Text("正在登录…".to_owned()));
        lines.push(Line::Action("取消登录".to_owned(), TAG_CANCEL_SIGN_IN));
    } else {
        lines.push(Line::Action("登录…".to_owned(), TAG_SIGN_IN));
    }
    lines.push(Line::Action("重新加载配置".to_owned(), TAG_RELOAD));
    lines.push(Line::Action("打开配置文件…".to_owned(), TAG_OPEN_CONFIG));
    lines
}

pub fn status_line(display: &Display) -> String {
    match display {
        Display::Unconfigured(reason) => reason.clone(),
        Display::SignedOut => "未登录".to_owned(),
        Display::SignedIn => "已登录".to_owned(),
        Display::Paused => "已暂停同步".to_owned(),
        Display::Sync { status, pending } => {
            let base = match status {
                Status::Connecting => "正在连接…".to_owned(),
                Status::Online => "已连接".to_owned(),
                Status::Offline(error) => format!("离线，稍后自动重试（{}）", short(error)),
                Status::Unauthorized => "登录已失效：请重新登录".to_owned(),
                Status::Disabled => "跨设备剪贴板在服务器上没开".to_owned(),
            };
            if *pending > 0 {
                format!("{base} · {pending} 条待上传")
            } else {
                base
            }
        }
    }
}

fn feature_title(feature: Feature) -> &'static str {
    match feature {
        Feature::Clipboard => "跨设备剪贴板",
        Feature::Sync => "同步学习数据与设置",
        Feature::InputLog => "上传输入日志",
        Feature::Llm => "大模型（云联想）",
    }
}

fn data_line(data: &DataStatus) -> String {
    let mut line = if let Some(error) = &data.error {
        format!("学习数据：同步失败，稍后重试（{}）", short(error))
    } else if data.waiting_for_ime {
        "学习数据：等输入法合并（切到青简打几个字）".to_owned()
    } else if let Some(ms) = data.last_ok_ms {
        format!("学习数据：{}同步", ago(ms))
    } else {
        "学习数据：正在同步…".to_owned()
    };
    if data.config_conflict {
        line.push_str(" · 设置有冲突，旧的一份已备份");
    }
    line
}

/// 「刚刚」/「N 分钟前」/「N 小时前」。
fn ago(ms: i64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default();
    let minutes = (now - ms).max(0) / 60_000;
    match minutes {
        0 => "刚刚".to_owned(),
        1..=59 => format!("{minutes} 分钟前"),
        _ => format!("{} 小时前", minutes / 60),
    }
}

/// 错误信息在菜单里只显示前 40 个字符，完整的在日志里。
fn short(text: &str) -> String {
    text.chars().take(40).collect()
}

#[cfg(test)]
mod tests {
    use qingjian_cloud_proto::Consents;

    use super::*;

    fn account(signed_in: bool) -> AccountMenu {
        AccountMenu {
            signed_in,
            signing_in: false,
            consents: Consents {
                sync: true,
                ..Consents::default()
            },
            note: None,
        }
    }

    #[test]
    fn toggle_tags_round_trip_and_stay_in_range() {
        for feature in Feature::ALL {
            let tag = toggle_tag(feature);
            assert!((-100..0).contains(&tag));
            assert_eq!(toggled_feature(tag), Some(feature));
        }
        for tag in [TAG_SIGN_OUT, TAG_CANCEL_SIGN_IN, TAG_PAUSE, 0, 5, -14] {
            assert_eq!(toggled_feature(tag), None);
        }
    }

    #[test]
    fn signed_out_menu_offers_sign_in_only() {
        let lines = build_lines(&Display::SignedOut, &account(false), None, &History::default());
        assert_eq!(lines[0], Line::Text("未登录".to_owned()));
        assert!(lines.contains(&Line::Action("登录…".to_owned(), TAG_SIGN_IN)));
        assert!(!lines.iter().any(
            |line| matches!(line, Line::Action(_, tag) if toggled_feature(*tag).is_some())
        ));
        assert!(!lines.contains(&Line::Action("暂停同步".to_owned(), TAG_PAUSE)));
    }

    #[test]
    fn signed_in_menu_shows_switch_states() {
        let lines = build_lines(&Display::SignedIn, &account(true), None, &History::default());
        assert!(lines.contains(&Line::Action(
            "同步学习数据与设置：开".to_owned(),
            toggle_tag(Feature::Sync)
        )));
        assert!(lines.contains(&Line::Action(
            "跨设备剪贴板：关".to_owned(),
            toggle_tag(Feature::Clipboard)
        )));
        assert!(lines.contains(&Line::Action("退出登录".to_owned(), TAG_SIGN_OUT)));
        // 剪贴板关着：不列历史
        assert!(!lines.contains(&Line::Text("还没有剪贴板记录".to_owned())));
    }

    #[test]
    fn signing_in_menu_can_cancel() {
        let mut menu = account(false);
        menu.signing_in = true;
        menu.note = Some("已取消登录".to_owned());
        let lines = build_lines(&Display::SignedOut, &menu, None, &History::default());
        assert!(lines.contains(&Line::Text("已取消登录".to_owned())));
        assert!(lines.contains(&Line::Action("取消登录".to_owned(), TAG_CANCEL_SIGN_IN)));
        assert!(!lines.contains(&Line::Action("登录…".to_owned(), TAG_SIGN_IN)));
    }
}
```

- [ ] **Step 9: `Service` 拆目录并接上账号**

```bash
cd /Users/liyuqing/sproot/qingjian-mainline/cloud/crates/qingjian-cloud-mac/src
mkdir service && git mv service.rs service/mod.rs
```

`src/service/mod.rs` 整个文件替换为：

```rust
//! 同步本体：主线程上 0.5 秒一拍，看本机剪贴板有没有变、取收到的事件、处理账号操作的结果、刷新菜单行。
//! 网络都在 `ClipboardSync` / `DataSync` 与账号操作的后台线程里，主线程从不等网络。
//! 跑在输入法进程里：入口都包 `catch_unwind`，这里出错只停同步，不能把输入法带崩（跨 ObjC 边界的 panic 会直接终止进程）。
//! 登录、退出登录、功能开关与跟随服务器状态在 `account.rs`。

mod account;

use std::cell::{Cell, RefCell};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::{Duration, Instant};

use objc2::rc::Retained;
use objc2::{MainThreadMarker, sel};
use objc2_foundation::NSTimer;
use qingjian_cloud_client::{ClipboardSync, DataSync, DataSyncConfig, SyncConfig};
use qingjian_cloud_proto::EventKind;

use crate::account::{AccountEvent, WebLogin};
use crate::config::AgentConfig;
use crate::history::History;
use crate::llm_endpoint::LlmEndpoint;
use crate::menu::{
    AccountMenu, Display, Line, TAG_CANCEL_SIGN_IN, TAG_OPEN_CONFIG, TAG_PAUSE, TAG_RELOAD,
    TAG_SIGN_IN, TAG_SIGN_OUT, TAG_SYNC_NOW, build_lines, status_line, toggled_feature,
};
use crate::timer::TimerTarget;
use crate::watcher::ClipboardWatcher;
use crate::{input_source, pasteboard, paths};

/// 主循环间隔（秒）。
const TICK: f64 = 0.5;

/// 每隔这么多拍看一次当前输入法（2 秒）。
const INPUT_SOURCE_EVERY: u32 = 4;

/// 连续这么久不是青简才暂停同步：密码框里系统会临时切到英文键盘，不能一进密码框就停。
const OTHER_INPUT_GRACE: Duration = Duration::from_secs(30);

/// 刚上传过或刚收到的同一段文字，这么久之内不再上传：和苹果通用剪贴板之间的最后一道防回灌。
const ECHO_WINDOW: Duration = Duration::from_secs(10 * 60);

/// 状态行没变也隔这么久重算一次菜单行：「N 分钟前同步」要跟着走。
const MENU_REFRESH: Duration = Duration::from_secs(30);

/// 别的设备复制的条目，在这么久以内收到才自动写进本机剪贴板（毫秒）。
/// 更早的（离线很久后补拉到的）只进菜单里的历史，免得突然覆盖用户正在用的剪贴板。
const AUTO_PASTE_WINDOW_MS: i64 = 120_000;

/// 主线程这边出错后，第一次重启前等多久，之后每次翻倍。
const FIRST_RESTART: Duration = Duration::from_secs(10);

/// 重启最长等多久。
const MAX_RESTART: Duration = Duration::from_secs(600);

thread_local! {
    static SERVICE: RefCell<Option<Service>> = const { RefCell::new(None) };

    /// 定时器单独放：服务出错被丢掉后它还在走，到点重建服务。
    static TIMER: RefCell<Option<Retained<NSTimer>>> = const { RefCell::new(None) };

    /// 服务出错停了：什么时候重建、这次等了多久（下次翻倍）。
    static RESTART: Cell<Option<(Instant, Duration)>> = const { Cell::new(None) };
}

/// 输入法启动时调一次：挂上定时器、读配置、起同步。重复调用不做事。
pub fn start(mtm: MainThreadMarker) {
    if TIMER.with(|timer| timer.borrow().is_some()) {
        return;
    }
    let target = TimerTarget::new(mtm);
    // NSTimer 持有 target，定时器活着它就活着
    let timer = unsafe {
        NSTimer::scheduledTimerWithTimeInterval_target_selector_userInfo_repeats(
            TICK,
            &target,
            sel!(tick:),
            None,
            true,
        )
    };
    TIMER.with(|cell| *cell.borrow_mut() = Some(timer));
    build(FIRST_RESTART);
}

/// 「青简 Cloud ›」子菜单的内容；没启动或出错停了时为空（输入法据此收起子菜单）。
pub fn menu_lines() -> Vec<Line> {
    with(|service| service.lines.clone()).unwrap_or_default()
}

/// 菜单行每变一次加一，输入法据此判断要不要重画子菜单。
pub fn menu_revision() -> u64 {
    with(|service| service.revision).unwrap_or(0)
}

/// 云联想选青简 Cloud 时用的大模型代理端点；没登录、没开大模型（或服务没起来）为 `None`。
pub fn llm_endpoint() -> Option<LlmEndpoint> {
    with(|service| service.endpoint.clone()).flatten()
}

/// 子菜单里点了一项。
pub fn perform(tag: isize) {
    with(|service| service.perform(tag));
}

pub(crate) fn tick() {
    if let Some((at, waited)) = RESTART.with(Cell::get) {
        if Instant::now() >= at {
            tracing::info!("青简 Cloud 重启");
            build((waited * 2).min(MAX_RESTART));
        }
        return;
    }
    with(Service::tick);
}

/// 建服务；出错就按 `next_wait` 排下一次重建。
fn build(next_wait: Duration) {
    match catch_unwind(AssertUnwindSafe(|| {
        let mut service = Service::new();
        service.refresh_menu(true);
        service
    })) {
        Ok(service) => {
            SERVICE.with(|cell| *cell.borrow_mut() = Some(service));
            RESTART.with(|cell| cell.set(None));
            tracing::info!("青简 Cloud 已启动");
        }
        Err(_) => schedule_restart(next_wait),
    }
}

fn schedule_restart(wait: Duration) {
    tracing::error!(
        wait_secs = wait.as_secs(),
        "青简 Cloud 出错，已停止同步，稍后自动重启（见上面的日志）"
    );
    RESTART.with(|cell| cell.set(Some((Instant::now() + wait, wait))));
}

/// 在主线程上取服务；重入时（正在处理上一个回调）跳过。出错就丢掉服务、排定重启，输入法照常打字。
fn with<T>(f: impl FnOnce(&mut Service) -> T) -> Option<T> {
    let (result, crashed) = SERVICE.with(|cell| {
        let Ok(mut guard) = cell.try_borrow_mut() else {
            return (None, false);
        };
        let Some(service) = guard.as_mut() else {
            return (None, false);
        };
        match catch_unwind(AssertUnwindSafe(|| f(service))) {
            Ok(value) => (Some(value), false),
            Err(_) => {
                // 丢掉服务时它的同步线程收到停止信号，各自退出
                *guard = None;
                (None, true)
            }
        }
    });
    if crashed {
        schedule_restart(FIRST_RESTART);
    }
    result
}

struct Service {
    /// 剪贴板同步；登录了、开了剪贴板且没暂停才有。
    sync: Option<ClipboardSync>,

    /// 学习数据、设置与输入日志的同步；没登录、两项都没开或暂停时为 `None`。
    data: Option<DataSync>,

    /// 大模型代理的地址与令牌，输入法的云联想选青简 Cloud 时用；没登录或没开大模型为 `None`。
    endpoint: Option<LlmEndpoint>,

    /// 读到的配置；读不了时为 `None`（原因在 `unconfigured`）。
    config: Option<AgentConfig>,

    /// 配置读不了或同步起不来的原因，菜单里显示。
    unconfigured: Option<String>,

    /// 用户在菜单里点了「暂停同步」。
    paused: bool,

    /// 当前输入法不是青简已超过 [`OTHER_INPUT_GRACE`]，同步线程都停了。
    suspended: bool,

    watcher: ClipboardWatcher,

    history: History,

    /// 给输入法画子菜单的行。
    lines: Vec<Line>,

    /// [`Self::lines`] 的版本号，变一次加一。
    revision: u64,

    /// 上次重算菜单行的时间。
    menu_built: Instant,

    /// 拍数，按 [`INPUT_SOURCE_EVERY`] 看当前输入法。
    ticks: u32,

    /// 最近一次上传或收到的文字的哈希与时间，防回灌用。
    last_synced: Option<(u64, Instant)>,

    /// 从什么时候起当前输入法不是青简。
    other_input_since: Option<Instant>,

    /// 开着的网页登录窗口；丢掉即取消登录、关窗。
    login: Option<WebLogin>,

    /// 登录窗已关、正在用一次性码换令牌。
    exchanging: bool,

    /// 账号操作的后台线程与登录窗口回调把结果发到这里，主线程每拍取。
    events: Receiver<AccountEvent>,

    sender: Sender<AccountEvent>,

    /// 最近一次登录或切换开关失败的原因，菜单里显示。
    note: Option<String>,
}

impl Service {
    fn new() -> Self {
        let (sender, events) = mpsc::channel();
        let mut service = Self {
            sync: None,
            data: None,
            endpoint: None,
            config: None,
            unconfigured: None,
            paused: false,
            suspended: false,
            watcher: ClipboardWatcher::new(),
            history: History::default(),
            lines: Vec::new(),
            revision: 0,
            menu_built: Instant::now(),
            ticks: 0,
            last_synced: None,
            other_input_since: None,
            login: None,
            exchanging: false,
            events,
            sender,
            note: None,
        };
        service.load_config();
        service.refresh_consents();
        service
    }

    fn load_config(&mut self) {
        // 先停旧的，再按新配置起；进度文件按服务器地址区分，换服务器会从头同步
        self.sync = None;
        self.data = None;
        self.endpoint = None;
        self.history = History::default();
        self.config = None;
        let (Some(config_path), Some(state_dir)) = (paths::config_path(), paths::support_dir())
        else {
            self.unconfigured = Some("找不到用户目录".to_owned());
            return;
        };
        let config = match AgentConfig::load(&config_path) {
            Ok(config) => config,
            Err(reason) => {
                tracing::info!(%reason, "青简 Cloud 配置读不了");
                self.unconfigured = Some(reason);
                return;
            }
        };
        self.unconfigured = None;
        // 没登录或暂停中（切走了输入法）只记下配置，不起同步
        if !config.signed_in() || self.suspended {
            self.config = Some(config);
            return;
        }
        let server = config.server();
        if config.llm {
            self.endpoint = Some(LlmEndpoint::new(&server, &config.token));
        }
        if (config.sync || config.logs)
            && let Some(ime_dir) = paths::ime_dir()
        {
            match DataSync::start(DataSyncConfig {
                server: server.clone(),
                token: config.token.clone(),
                ime_dir,
                state_dir: state_dir.join("data"),
                sync_learning: config.sync,
                sync_logs: config.logs,
                log_download_dir: (config.logs && config.download_logs)
                    .then(|| state_dir.join("input-log")),
                sync_config: config.sync,
            }) {
                Ok(data) => self.data = Some(data),
                Err(error) => tracing::warn!(%error, "学习数据同步启动失败"),
            }
        }
        if config.clipboard {
            match ClipboardSync::start(SyncConfig {
                server,
                token: config.token.clone(),
                state_dir,
            }) {
                Ok(sync) => self.sync = Some(sync),
                Err(error) => {
                    tracing::warn!(%error, "剪贴板同步启动失败");
                    self.unconfigured = Some(format!("同步启动失败：{error}"));
                }
            }
        }
        self.config = Some(config);
    }

    /// 每拍：处理账号操作的结果，看当前输入法决定暂停 / 恢复，跟着服务器状态改开关，
    /// 上传本机新复制的、写入别的设备刚复制的、刷新菜单行。
    fn tick(&mut self) {
        self.ticks = self.ticks.wrapping_add(1);
        // 登录窗的回调、换令牌的结果不能等到切回青简
        self.apply_account_events();
        if self.ticks.is_multiple_of(INPUT_SOURCE_EVERY) {
            self.follow_input_source();
        }
        if self.suspended {
            return;
        }
        self.follow_server_state();
        let copied = self.watcher.poll();
        let Some(sync) = &self.sync else {
            self.refresh_menu(false);
            return;
        };
        if let Some(text) = copied
            && !self.paused
        {
            let hash = text_hash(&text);
            if self.recently_synced(hash) {
                tracing::debug!("与刚同步过的内容相同，不再上传");
            } else {
                self.last_synced = Some((hash, Instant::now()));
                sync.copy(text);
            }
        }
        let mut changed = false;
        let now = now_ms();
        while let Some(incoming) = sync.try_recv() {
            changed |= self.history.apply(&incoming);
            if let EventKind::ClipAdded { text, .. } = &incoming.event.kind
                && !incoming.mine
                && !self.paused
                && (now - incoming.event.at).abs() < AUTO_PASTE_WINDOW_MS
            {
                self.last_synced = Some((text_hash(text), Instant::now()));
                // 通用剪贴板正管着剪贴板（同一 Apple ID 的设备刚复制过）：交给它，不写也不读（读会跨设备取数据卡主线程）
                if pasteboard::is_remote() {
                    tracing::debug!(seq = incoming.event.seq, "剪贴板是通用剪贴板送来的，不写");
                    continue;
                }
                // 已经一样就不写，写了会再被通用剪贴板广播回去
                if pasteboard::current_text().as_deref() == Some(text.as_str()) {
                    tracing::debug!(seq = incoming.event.seq, "剪贴板里已是这段文字，不写");
                    continue;
                }
                let count = pasteboard::write_text(text);
                self.watcher.note_own_write(count);
                tracing::info!(seq = incoming.event.seq, device = %incoming.event.device, "写入剪贴板");
            }
        }
        self.refresh_menu(changed);
    }

    /// 只在用青简时同步：切走超过 [`OTHER_INPUT_GRACE`] 就停掉同步线程，切回来按配置重新起。
    fn follow_input_source(&mut self) {
        match input_source::qingjian_selected() {
            Some(false) => {
                let since = *self.other_input_since.get_or_insert_with(Instant::now);
                if !self.suspended && since.elapsed() >= OTHER_INPUT_GRACE {
                    tracing::info!("切到了别的输入法，青简 Cloud 暂停同步");
                    self.suspended = true;
                    self.sync = None;
                    self.data = None;
                }
            }
            Some(true) => {
                self.other_input_since = None;
                if self.suspended {
                    tracing::info!("切回青简，青简 Cloud 恢复同步");
                    self.suspended = false;
                    // 暂停期间的复制不补传：从当前剪贴板重新开始看
                    self.watcher = ClipboardWatcher::new();
                    self.load_config();
                    self.refresh_menu(true);
                }
            }
            None => {}
        }
    }

    /// 子菜单的动作。
    fn perform(&mut self, tag: isize) {
        if let Some(feature) = toggled_feature(tag) {
            self.toggle(feature);
            return;
        }
        match tag {
            TAG_PAUSE => {
                self.paused = !self.paused;
                self.refresh_menu(true);
            }
            TAG_SYNC_NOW => {
                if let Some(data) = &self.data {
                    data.sync_now();
                }
            }
            TAG_RELOAD => {
                self.load_config();
                self.refresh_consents();
                self.refresh_menu(true);
            }
            TAG_OPEN_CONFIG => {
                if let Some(path) = paths::config_path() {
                    if !path.exists() {
                        let _ = AgentConfig::load(&path);
                    }
                    open(&["-t", &path.to_string_lossy()]);
                }
            }
            TAG_SIGN_IN => self.sign_in(),
            TAG_CANCEL_SIGN_IN => self.cancel_sign_in(),
            TAG_SIGN_OUT => self.sign_out(),
            index if index >= 0 => {
                if let Some(entry) = self.history.get(index as usize) {
                    let count = pasteboard::write_text(&entry.text);
                    self.watcher.note_own_write(count);
                }
            }
            _ => {}
        }
    }

    fn recently_synced(&self, hash: u64) -> bool {
        self.last_synced
            .is_some_and(|(last, at)| last == hash && at.elapsed() < ECHO_WINDOW)
    }

    fn signed_in(&self) -> bool {
        self.config.as_ref().is_some_and(AgentConfig::signed_in)
    }

    fn display(&self) -> Display {
        if let Some(reason) = &self.unconfigured {
            return Display::Unconfigured(reason.clone());
        }
        if !self.signed_in() {
            return Display::SignedOut;
        }
        if self.paused {
            return Display::Paused;
        }
        match &self.sync {
            Some(sync) => Display::Sync {
                status: sync.status(),
                pending: sync.pending(),
            },
            None => Display::SignedIn,
        }
    }

    /// 强制、状态行变了、或隔了 [`MENU_REFRESH`] 才重算；算出来和上次一样就不动版本号。
    fn refresh_menu(&mut self, force: bool) {
        let display = self.display();
        let status_changed = self.lines.first() != Some(&Line::Text(status_line(&display)));
        if !force && !status_changed && self.menu_built.elapsed() < MENU_REFRESH {
            return;
        }
        let account = AccountMenu {
            signed_in: self.signed_in(),
            signing_in: self.login.is_some() || self.exchanging,
            consents: self
                .config
                .as_ref()
                .map(AgentConfig::consents)
                .unwrap_or_default(),
            note: self.note.clone(),
        };
        let data = self.data.as_ref().map(DataSync::status);
        let lines = build_lines(&display, &account, data.as_ref(), &self.history);
        self.menu_built = Instant::now();
        if lines != self.lines {
            self.lines = lines;
            self.revision += 1;
        }
    }
}

fn text_hash(text: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

fn open(args: &[&str]) {
    if let Err(error) = std::process::Command::new("open").args(args).spawn() {
        tracing::warn!(%error, ?args, "open 失败");
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or_default()
}
```

`src/service/account.rs`：

```rust
//! 账号：网页登录、用一次性码换令牌、退出登录、切换功能开关、跟着服务器的状态改本机开关。
//! 网络请求都在一次性的后台线程里，结果经通道回到主线程拍子（[`Service::apply_account_events`]）。

use std::path::Path;

use objc2::MainThreadMarker;
use qingjian_cloud_client::{Client, ClientError, ClipboardSync, DataSync, Status};
use qingjian_cloud_proto::{Consents, Device, Feature, HandoffExchange, Platform};

use super::Service;
use crate::account::{self, AccountEvent, WebLogin};
use crate::config::AgentConfig;
use crate::paths;

impl Service {
    /// 「登录…」：生成 verifier，打开网页登录窗口。
    pub(super) fn sign_in(&mut self) {
        if self.login.is_some() || self.exchanging {
            return;
        }
        let (Some(config), Some(mtm)) = (&self.config, MainThreadMarker::new()) else {
            return;
        };
        let verifier = match account::new_verifier() {
            Ok(verifier) => verifier,
            Err(reason) => {
                self.note = Some(reason);
                self.refresh_menu(true);
                return;
            }
        };
        let device = account::device_name();
        let url = account::login_url(&config.server(), &account::challenge(&verifier), &device);
        match WebLogin::start(mtm, &url, verifier, device, self.sender.clone()) {
            Ok(login) => {
                tracing::info!("打开网页登录");
                self.login = Some(login);
                self.note = None;
            }
            Err(reason) => self.note = Some(reason),
        }
        self.refresh_menu(true);
    }

    /// 「取消登录」：丢掉登录窗口（取消会话、关窗），不再等换令牌的结果。
    pub(super) fn cancel_sign_in(&mut self) {
        self.login = None;
        self.exchanging = false;
        self.refresh_menu(true);
    }

    /// 「退出登录」：本机立即退出；服务器上的注销在后台发，失败只记日志。
    pub(super) fn sign_out(&mut self) {
        let Some(config) = self.config.as_ref().filter(|config| config.signed_in()) else {
            return;
        };
        let client = Client::new(&config.server(), &config.token);
        spawn("cloud-sign-out", move || {
            if let Err(error) = client.sign_out() {
                tracing::warn!(%error, "服务器上退出登录失败，本机已退出");
            }
        });
        self.note = None;
        self.store(AgentConfig::clear_session);
        tracing::info!("青简 Cloud 已退出登录");
        self.refresh_menu(true);
    }

    /// 切换一项功能：先告诉服务器，按服务器回的四项开关改本机。
    pub(super) fn toggle(&mut self, feature: Feature) {
        let Some(config) = self.config.as_ref().filter(|config| config.signed_in()) else {
            return;
        };
        let enabled = !config.consents().get(feature);
        let client = Client::new(&config.server(), &config.token);
        let sender = self.sender.clone();
        self.note = None;
        spawn("cloud-consent", move || {
            let event = match client.put_consent(feature, enabled) {
                Ok(consents) => AccountEvent::Consents(consents),
                Err(ClientError::Forbidden(_)) => AccountEvent::Forbidden(feature),
                Err(ClientError::Unauthorized) => AccountEvent::SignedOut,
                Err(error) => {
                    AccountEvent::Failed(format!("开关没改成：{}", account::reason(&error)))
                }
            };
            let _ = sender.send(event);
        });
    }

    /// 启动与「重新加载配置」时问一次服务器上的开关：别的设备改过的跟过来。
    pub(super) fn refresh_consents(&self) {
        let Some(config) = self.config.as_ref().filter(|config| config.signed_in()) else {
            return;
        };
        let client = Client::new(&config.server(), &config.token);
        let sender = self.sender.clone();
        spawn("cloud-account", move || {
            let event = match client.account() {
                Ok(account) => AccountEvent::Consents(account.consents),
                Err(ClientError::Unauthorized) => AccountEvent::SignedOut,
                Err(error) => {
                    tracing::info!(%error, "取账号失败，开关照本机配置");
                    return;
                }
            };
            let _ = sender.send(event);
        });
    }

    pub(super) fn apply_account_events(&mut self) {
        let mut changed = false;
        while let Ok(event) = self.events.try_recv() {
            self.apply(event);
            changed = true;
        }
        if changed {
            self.refresh_menu(true);
        }
    }

    /// 同步线程报 401 / 403：令牌失效就退出登录，某项功能服务器上没开就把本机开关记成关。
    pub(super) fn follow_server_state(&mut self) {
        let clipboard = self.sync.as_ref().map(ClipboardSync::status);
        let data = self.data.as_ref().map(DataSync::status);
        if clipboard == Some(Status::Unauthorized)
            || data.as_ref().is_some_and(|data| data.unauthorized)
        {
            self.apply(AccountEvent::SignedOut);
            self.refresh_menu(true);
            return;
        }
        let mut off = Vec::new();
        if clipboard == Some(Status::Disabled) {
            off.push(Feature::Clipboard);
        }
        if let Some(data) = &data {
            off.extend(data.disabled.iter().copied());
        }
        if !off.is_empty() {
            self.switch_off(&off);
            self.refresh_menu(true);
        }
    }

    fn apply(&mut self, event: AccountEvent) {
        match event {
            AccountEvent::Callback(Ok(url)) => self.exchange(&url),
            AccountEvent::Callback(Err(reason)) => {
                self.login = None;
                self.note = Some(reason);
            }
            AccountEvent::SignedIn { token, consents } => {
                self.exchanging = false;
                self.note = None;
                // 先停同步线程再删它们的进度文件
                self.sync = None;
                self.data = None;
                reset_sync_state();
                self.store(|path| AgentConfig::store_session(path, &token, consents));
                tracing::info!("青简 Cloud 已登录");
            }
            AccountEvent::Consents(consents) => {
                if self.config.as_ref().map(AgentConfig::consents) != Some(consents) {
                    self.forget_clipboard_queue_if_off(consents);
                    self.store(|path| AgentConfig::store_consents(path, consents));
                }
            }
            AccountEvent::Forbidden(feature) => {
                self.note = Some("这项功能现在不能打开".to_owned());
                self.switch_off(&[feature]);
            }
            AccountEvent::SignedOut => {
                self.login = None;
                self.exchanging = false;
                self.note = Some("登录已失效，请重新登录".to_owned());
                self.store(AgentConfig::clear_session);
            }
            AccountEvent::Failed(reason) => {
                self.exchanging = false;
                self.note = Some(reason);
            }
        }
    }

    /// 登录窗回跳：关掉窗口，在后台用一次性码与 verifier 换令牌，再问一次服务器上的开关。
    fn exchange(&mut self, url: &str) {
        let Some(login) = self.login.take() else {
            return;
        };
        let Some(handoff) = account::handoff_from_callback(url) else {
            self.note = Some("登录回调里没有一次性码，请重试".to_owned());
            return;
        };
        let Some(server) = self.config.as_ref().map(AgentConfig::server) else {
            return;
        };
        let request = HandoffExchange {
            handoff,
            verifier: login.verifier().to_owned(),
            device: Device {
                name: login.device().to_owned(),
                platform: Platform::Macos,
            },
        };
        drop(login);
        self.exchanging = true;
        let sender = self.sender.clone();
        spawn("cloud-login", move || {
            let event = match Client::anonymous(&server).exchange_handoff(&request) {
                Ok(grant) => {
                    let consents = Client::new(&server, &grant.token)
                        .account()
                        .map(|account| account.consents)
                        .inspect_err(|error| tracing::warn!(%error, "登录后取开关失败，先全关"))
                        .unwrap_or_default();
                    AccountEvent::SignedIn {
                        token: grant.token,
                        consents,
                    }
                }
                Err(error) => AccountEvent::Failed(format!("登录失败：{}", account::reason(&error))),
            };
            let _ = sender.send(event);
        });
    }

    /// 把这几项本机开关记成关。
    fn switch_off(&mut self, features: &[Feature]) {
        let Some(mut consents) = self.config.as_ref().map(AgentConfig::consents) else {
            return;
        };
        for &feature in features {
            consents.set(feature, false);
        }
        tracing::info!(?features, "服务器上没开，本机开关记成关");
        self.forget_clipboard_queue_if_off(consents);
        self.store(|path| AgentConfig::store_consents(path, consents));
    }

    /// 剪贴板关掉时扔掉还没发出去的队列：再打开时不该把关着之前复制的旧内容补发出去。
    fn forget_clipboard_queue_if_off(&mut self, consents: Consents) {
        if consents.clipboard {
            return;
        }
        // 先停剪贴板线程，它握着队列文件
        self.sync = None;
        if let Some(dir) = paths::support_dir() {
            remove(&dir.join("outbox.jsonl"));
        }
    }

    /// 改配置文件再按新配置重起同步；写不进去把原因显示在菜单里。
    fn store(&mut self, write: impl FnOnce(&Path) -> Result<(), String>) {
        match paths::config_path() {
            Some(path) => {
                if let Err(reason) = write(&path) {
                    tracing::warn!(%reason, "青简 Cloud 配置写不进去");
                    self.note = Some(reason);
                }
            }
            None => self.note = Some("找不到用户目录".to_owned()),
        }
        self.load_config();
    }
}

/// 换了账号：剪贴板游标与设备名、离线队列、学习数据基线、下载的输入日志都属于上一个账号，删掉重来。
/// 基线留着的话，新账号服务器上是空的，算出来「别的设备的增量」是负的，会把本机学到的减掉。
fn reset_sync_state() {
    let Some(dir) = paths::support_dir() else {
        return;
    };
    for name in ["state.json", "outbox.jsonl", "data", "input-log"] {
        remove(&dir.join(name));
    }
}

fn remove(path: &Path) {
    let result = if path.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    };
    if let Err(error) = result
        && error.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(%error, path = %path.display(), "删不掉");
    }
}

fn spawn(name: &str, work: impl FnOnce() + Send + 'static) {
    if let Err(error) = std::thread::Builder::new()
        .name(name.to_owned())
        .spawn(work)
    {
        tracing::warn!(%error, thread = name, "后台线程起不来");
    }
}
```

- [ ] **Step 10: 全部测试与检查**

Run:
```bash
cd /Users/liyuqing/sproot/qingjian-mainline/cloud
cargo test -p qingjian-cloud-mac
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p qingjian-cloud-mac --example run -- 3
```
Expected: `account::tests` 5 个、`config::tests` 4 个、`menu::tests` 4 个与原有 `llm_endpoint` 测试全部 PASS；clippy 无警告；`example run` 打印的子菜单行里第一行是 `Text("未登录")`（本机 config.toml 还是旧 `qjc_` 令牌或没有令牌时），有 `Action("登录…", -7)`。

- [ ] **Step 11: 输入法侧的 Cargo.lock**

输入法（仓库根 workspace 的 `qingjian-macos`）按路径依赖 `qingjian-cloud-mac`，新依赖要进根 `Cargo.lock`：

```bash
cd /Users/liyuqing/sproot/qingjian-mainline
cargo build -p qingjian-macos
git diff --stat Cargo.lock
```

Expected: 编译通过，`Cargo.lock` 多了 `objc2-authentication-services`、`block2`、`base64`、`percent-encoding`、`getrandom` 等条目（`sha2 0.11`、`toml_edit 0.25` 已有则不变）。
`cloud/docs/fork-patch.md` 第 116 行：

```
| `Cargo.lock` | 新增条目 | ureq、native-tls 证书等；`uuid` 1.26.1 → 1.27.0（cloud 要求） |
```

改成：

```
| `Cargo.lock` | 新增条目 | ureq、native-tls 证书等；`uuid` 1.26.1 → 1.27.0（cloud 要求）；2026-10-04 账号登录加 objc2-authentication-services、block2、base64、getrandom、percent-encoding、toml_edit |
```

- [ ] **Step 12: 更新 Mac 端 README**

`cloud/crates/qingjian-cloud-mac/README.md`：第 1 段里 `本机复制的文本上传到你的服务器` 改成 `本机复制的文本上传到素笺服务器`；`- **大模型**：…` 那一条整条换成：

```markdown
- **大模型**：「大模型（云联想）」开着时，输入法的云联想（`[predict] provider = qingjian`）走素笺服务器的大模型代理，Mac 上不需要大模型密钥。
```

「## 安装与配置」整节（从标题到 `日志写在输入法的日志里（\`~/Library/Logs/Qingjian/\`）。` 那一行）替换为：

````markdown
## 安装与登录

装 `cloud/scripts/publish-mac.sh` 发的输入法 pkg（或 `sujian` 分支的检出里 `apps/macos/scripts/bundle.sh --install`），同步模块就在里面。
从旧版（单独的 `QingjianCloud.app` 常驻程序）升级时，pkg 会把它和它的登录项删掉。

「中☁ → 青简 Cloud ›」里点「登录…」，会弹出素笺的登录页（用 Apple ID 或邮箱验证码），登录完自动回到输入法，菜单变成「已登录」。
四项功能缺省都关，在同一个子菜单里逐项打开：跨设备剪贴板、同步学习数据与设置、上传输入日志、大模型（云联想）。
切换时先告诉服务器，服务器那边关掉某项会同时删除这部分云端数据；在 iPhone 上关掉的，Mac 这边几秒内也跟着显示为关。
「退出登录」立即生效。换了账号登录时，本机旧的同步进度（剪贴板游标、离线队列、学习数据基线）会清掉重来。

配置在 `~/Library/Application Support/QingjianCloud/config.toml`（仅本人可读），一般不用手改：

```toml
server = "https://pinyin.synon.ai"
token = ""          # 由「登录…」写入，不要手填
clipboard = false   # 以下四项在菜单里切换
sync = false
logs = false
llm = false
download_logs = true
```

旧版手填的 `qjc_` 设备令牌已经作废，升级后显示「未登录」，重新登录即可。这份配置不并进输入法的 `config.toml`：那份会同步到别的设备，令牌是每台设备自己的。
日志写在输入法的日志里（`~/Library/Logs/Qingjian/`）。
````

三节验收清单里「填好各自的令牌」「配好不同设备的令牌」改成「用同一个账号登录并打开对应功能」；「服务器上 `qingjian-cloud usage`」改成「服务器上 `sujian-admin usage`」；
「`qingjian-cloud export-log --device <设备名>` 能看到这些上屏」整句删掉，改成「服务器上该用户的输入日志有这些上屏（`sujian-admin user list` 看会话数与最近活跃）」；第 5 条里 `后，服务器上 \`export-log\` 为空` 改成 `后，服务器上该用户的输入日志为空`。

- [ ] **Step 13: 真机验证（要等服务端部署，网页登录页与 Services ID 配好）**

1. `cd /Users/liyuqing/sproot/qingjian-mainline && apps/macos/scripts/bundle.sh --install`，切到青简。「中☁ → 青简 Cloud ›」第一行「未登录」，有「登录…」。
2. 点「登录…」：弹出登录窗口（可能先有系统的「“青简”想使用 pinyin.synon.ai 登录」确认框，点继续），页面上有 Apple 与邮箱两种。
   若菜单里出现「登录窗口出错：…」（锚点被判无效，错误码 3），把 `anchor.rs` 里的 `setAlphaValue(0.0)` 改成 `1.0`、样式改成 `NSWindowStyleMask::Titled` 再试，并把结论记进 `cloud/docs/design.md`。
3. 用邮箱登录：收码、输码，窗口自动关闭，菜单变「已登录」，四个开关全是「关」；`config.toml` 里 `token = "sjt_…"`、权限 `-rw-------`。
4. 点「跨设备剪贴板：关」→ 变「开」，第一行变「已连接」；复制一段字，iPhone（同账号、开了剪贴板）键盘弹出时提示这段字。
5. 在 iPhone 账号页关掉「跨设备剪贴板」：Mac 几秒内（SSE 收到 403）菜单变「跨设备剪贴板：关」，`~/Library/Application Support/QingjianCloud/outbox.jsonl` 不在了。
6. 打开「大模型（云联想）」，打一段拼音停一下，候选里出现云端词；关掉后不再出现（`llm_endpoint()` 为空，偏好设置里会提示青简 Cloud 没配置）。
7. 「退出登录」：菜单回到「未登录」，`config.toml` 的 token 清空、开关全关。再用 Apple 登录走一遍。
8. 登录窗口里点取消：菜单显示「已取消登录」，「登录…」可再点。
9. 关 Wi-Fi 打字：打字不受影响，没有弹窗。

- [ ] **Step 14: 提交**

```bash
cd /Users/liyuqing/sproot/qingjian-mainline
git add cloud/crates/qingjian-cloud-mac cloud/Cargo.lock Cargo.lock cloud/docs/fork-patch.md
git commit -m "$(cat <<'EOF'
feat(cloud): Mac 端改用网页登录拿会话令牌，四个功能开关缺省关

「青简 Cloud ›」加登录… / 取消登录 / 退出登录与四个开关；登录用 ASWebAuthenticationSession 打开服务端登录页，
PKCE verifier 只在进程里，回跳 sujian://auth?handoff=… 后换令牌写进 config.toml（toml_edit 保留注释，权限 600）。
输入法进程没有常驻窗口，临时建透明的 1×1 窗口当展示锚点；用旧的 callbackURLScheme 初始化以支持 macOS 13。
开关先 PUT 服务器，同步线程报 403 时本机开关记成关并扔掉剪贴板离线队列，报 401 时退出登录；换账号清掉旧的同步进度。

EOF
)"
```

---

## Task 6: 文档

**Files:**
- Modify: `cloud/docs/design.md`、`cloud/ios/README.md`、`cloud/README.md`

- [ ] **Step 1: `cloud/docs/design.md`**

1. 开头的引用块（第 3–4 行）后面加一段：

```markdown
> 2026-10-04：素笺子项目 1。青简 Cloud 从「一人一台自建服务器、设备令牌」改成素笺的多用户服务（`https://pinyin.synon.ai`）：
> Apple ID 或邮箱登录拿会话令牌，四项云功能各自同意、缺省全关，服务端按用户加密存储。下文「定位」「鉴权」两处以此为准；
> 服务端设计见 synon-ime 的 `docs/superpowers/specs/2026-10-04-account-multitenant-design.md`，客户端计划见 [plans/2026-10-04-account-client.md](plans/2026-10-04-account-client.md)。
```

2. 第 6 段 `青简 Cloud 是青简输入法的个人扩展：一台自建服务器，配上 Mac 端的一个常驻小程序和一个 iOS 键盘。` 替换为：

```markdown
青简 Cloud 是素笺（青简输入法的云端扩展）的客户端一侧：素笺的多用户服务器，配上链进 Mac 输入法进程的同步模块和一个 iOS 键盘。
```

3. 「## 定位」下第一条（`- **一人一台服务器。** …数据明文保存，也同步完整的输入历史。` 两行）替换为：

```markdown
- **素笺多用户服务。** 服务端闭源（synon-ime），部署在 `pinyin.synon.ai`；用户用 Apple ID 或邮箱登录，账号之间数据隔离。
  敏感字段（剪贴板、学习数据、输入日志、配置）按用户密钥加密入库，删账号即销毁密钥；服务端处理请求时能解密，这一点写进隐私政策。
  剪贴板、同步、输入日志、大模型四项各自单独同意、缺省全关；关掉一项即删除云端这部分数据，本机数据不动。
```

4. 「## 组成」里 `- **鉴权**：每台设备一个令牌，…吊销一台设备不影响其他设备。` 两行替换为：

```markdown
- **鉴权**：账号登录。iOS 在主 App 的「账号」页用 Sign in with Apple（带 nonce）或邮箱 6 位验证码登录，由 Rust 桥发请求，
  会话令牌（`sjt_…`）只写进 App Group 的 `cloud.toml`、不交给 Swift；Mac 在输入法菜单里点「登录…」，用 `ASWebAuthenticationSession`
  打开服务端的网页登录页，PKCE（verifier 只在输入法进程里）换回一次性码，再换会话令牌写进 `QingjianCloud/config.toml`。
  服务端只存令牌的哈希，请求时带在 `Authorization: Bearer …` 头里；每台设备一个会话，可以在任意设备上注销别的设备。
  构建产物里不带令牌，iOS 安装包只写死服务器地址。旧的 `qjc_` 设备令牌作废。
- **功能开关**：`PUT /v1/consents/{feature}`，四项（`clipboard` / `sync` / `input_log` / `llm`）新用户全关。客户端切换时先问服务器，
  成功后写回本机配置；同步线程收到 403（这项在别的设备上被关了）就停掉那一项、把本机开关记成关，收到 401 就退出登录。
  换账号登录时清掉本机旧的同步进度：学习数据基线留着的话，新账号服务器上是空的，会把本机学到的减掉。
```

5. 「### 实时：代理」里 `- 输入法这边**不用改代码**：Mac 端（\`qingjian-cloud-mac\`） 菜单里「让青简使用 Cloud 的大模型」…每台设备用的都是自己的令牌。现有的云端词与整句补全直接可用。` 整条替换为：

```markdown
- 输入法这边：`[predict] provider = qingjian`（缺省）时，地址与令牌在运行时从 `qingjian_cloud_mac::llm_endpoint` 拿，
  只在登录了且「大模型（云联想）」开着时才有；令牌是这台设备的会话令牌，不写进会同步的 `config.toml`。iOS 同理读 `cloud.toml` 的 `llm`。
```

   同一节 `- **记账**：按设备、按天记录 token 用量。` 改成 `- **记账**：按用户、按天记录 token 用量，超过每日上限返回 429。`

6. 「### 后台：纠错闭环」标题下第一段前插入一行：

```markdown
> 2026-10-04 起停用（部署里不起它），要改成按用户跑再恢复（素笺子项目 4）。
```

7. 「## 分期」表格末尾加一行：

```markdown
| 素笺 1 | 账号与多租户：Apple / 邮箱登录、会话、四项功能开关、按用户加密；iOS 账号页、Mac 网页登录 | 两台设备用同一账号登录后剪贴板与学习数据互通，另一个账号看不到 |
```

- [ ] **Step 2: `cloud/ios/README.md`**

先看 `git diff cloud/ios/README.md`：应当没有别人的未提交改动（写计划时 `sujian` 分支上这份文件是干净的）；若有，问用户是否一起提交。

1. 第 4 行 `「完全访问」用于按键震动（iOS 规定第三方键盘没有它不能震）与连青简 Cloud；不开照常离线打字、只有系统键盘音。` 改成
   `「完全访问」用于按键震动（iOS 规定第三方键盘没有它不能震）与登录后连素笺服务器；不开照常离线打字、只有系统键盘音。`
2. 「## 构建」里 `Keyboard target 的 preBuildScript 会跑 \`scripts/build-bridge.sh\`，它做两件事：` 那个列表加第三条：

```markdown
- 往 `Keyboard/Data/cloud.toml` 写服务器地址（缺省 `https://pinyin.synon.ai`，环境变量 `QJ_SERVER` 可换），**不带令牌**：构建与安装都不需要令牌，令牌由主 App 的账号页登录写入。
```

   该节末尾（`…并检查键盘签名里带了 App Group）。` 之后）加一段：

```markdown
App target 带 Sign in with Apple 权限（`com.apple.developer.applesignin`），自动签名会给 App ID `app.qingjian.cloud` 打开这项能力。
**Apple 登录只能在真机上测**；模拟器里能看到账号页、能走邮箱登录。`install-device.sh` 发现包里的 `cloud.toml` 带令牌会拒装。
以前用过的 `cloud.local.toml`（带旧的 `qjc_` 令牌）已作废，删掉即可。
```

3. 「## 设置」里 `主 App 首页有「键盘设置」与「青简 Cloud」两页` 改成 `主 App 首页有「键盘设置」与「账号」两页`；`- **青简 Cloud**：…键盘下次弹出时发现文件变了就重开会话。` 整条替换为：

```markdown
- **账号**：没登录时有「通过 Apple 登录」（`SignInWithAppleButton`，原始 nonce 交给桥、请求里给 Apple 的是它的 SHA-256）与「用邮箱登录」（邮箱 → 验证码）。
  登录后显示登录方式、设备列表（本机标出、别的设备可注销）、四个功能开关（缺省全关，打开时先问服务器，失败就弹回去）、退出登录与删除账号（二次确认）。
  这些都由桥（`qj_account_*`）发请求，令牌只在桥与 App Group 的 `cloud.toml` 之间流转；开关成功后桥写回 `cloud.toml`，键盘下次弹出时发现文件变了就重开会话。
```

4. 「## 连青简 Cloud」标题改成「## 登录之后」，第一段（`把 \`cloud.example.toml\` 复制成 \`cloud.local.toml\`…重新构建即可；没有这个文件键盘完全离线。连上之后：`）替换为：

```markdown
没登录时键盘完全离线（`cloud.toml` 里没有 `sjt_` 令牌，桥不建任何网络客户端、不记输入日志）。登录并打开对应开关之后：
```

   列表里 `- **云联想**：…` 改为以 `- **云联想**（「大模型」开关）：` 开头；`- **润色**：` 改为 `- **润色**（「大模型」开关）：`；`- **跨设备剪贴板**：` 保持；
   `- **学习数据同步**：` 改为 `- **学习数据同步**（「同步」开关）：`，这一条末尾 `别的设备的输入日志不下载。` 改成 `输入日志跟「上传输入日志」开关走，别的设备的不下载。换账号登录时 \`cloud/\` 下的同步进度清掉重来。`
   该节最后一行 `\`cloud.local.toml\` 构建时打进键盘，只作种子：主 App 第一次打开时拷进 App Group，之后以设置页里改的为准。` 替换为：

```markdown
随包的 `cloud.toml` 只有服务器地址，主 App 第一次打开时拷进 App Group；令牌与开关之后由账号页写入（格式见 `cloud.example.toml`）。
```

- [ ] **Step 3: `cloud/README.md`**

第 3 行 `青简输入法的个人扩展：自建服务器、跨设备剪贴板、输入历史汇总，以及用大模型补候选、修正词库、做联想。支持 macOS 与 iOS。` 改成：

```markdown
素笺（青简输入法的云端扩展）的客户端：用 Apple ID 或邮箱登录后，跨设备剪贴板、学习数据与设置同步、输入历史汇总，以及用大模型补候选、做润色，各项单独开启、缺省全关。支持 macOS 与 iOS。
```

- [ ] **Step 4: 自查文档里不再有手填令牌的说法**

Run: `cd /Users/liyuqing/sproot/qingjian-mainline/cloud && grep -rn "device add\|qjc_\|cloud.local.toml\|设备令牌" README.md docs/design.md ios/README.md crates/qingjian-cloud-mac/README.md`
Expected: 只剩「旧的 `qjc_` 设备令牌作废」「`cloud.local.toml`…已作废」这类说明性的句子，没有教人去生成或填令牌的步骤。

- [ ] **Step 5: 提交**

```bash
cd /Users/liyuqing/sproot/qingjian-mainline
git add cloud/docs/design.md cloud/ios/README.md cloud/README.md
git commit -m "$(cat <<'EOF'
docs(cloud): 定位改成素笺多用户服务，鉴权改成账号登录

design.md 的定位、鉴权、大模型代理改成账号登录与四项功能开关（缺省关），tuner 标停用；
iOS README 写明构建与安装不再需要令牌、Apple 登录只能真机，设置页改成账号页。

EOF
)"
```

---

## 自查（写计划时已核对）

- spec 第 7 节逐条对应：proto（Task 1）、client 的 `anonymous` 与 8 个方法、`Forbidden` / `RateLimited`、各同步线程 403 停下上报（Task 2）、bridge 开关缺省 false、C ABI 登录 / 账号 / 开关 / 退出 / 删账号、令牌写进 App Group 不出桥（Task 3）、iOS 账号页、Sign in with Apple 权限、构建脚本不拷 `cloud.local.toml`、`cloud.example.toml` 删令牌、键盘不改没登录离线（Task 4）、Mac token 由登录写入、「登录…」「退出登录」、四个开关缺省关、打开时 `put_consent`、403 显示为关（Task 5）、文档（Task 6）。
- 类型与路径与契约逐字一致：`Platform` / `Device` / `AppleClient` / `AppleSignIn` / `EmailStart` / `EmailVerify` / `HandoffExchange` / `HandoffGrant` / `SessionGrant` / `Feature` / `Consents` / `PutConsent` / `IdentityInfo` / `SessionInfo` / `Account`，9 个路径常量与 `TOKEN_PREFIX = "sjt_"`；`Client` 方法签名与 `ClientError` 映射照契约。
- 时间戳按服务端计划用 Unix 毫秒（`crate::now_ms()`），iOS 设备列表据此换算。
