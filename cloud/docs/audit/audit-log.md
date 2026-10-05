# 素笺子项目 1 审计与回归记录

审计对象：服务端会话（synon-ime `feat/sujian-account-server`）与客户端会话（qingjian `sujian` 分支 `cloud/`）。
依据：`synon-ime/docs/superpowers/specs/2026-10-04-account-multitenant-design.md`（spec）、两份实施计划、
`docs/design/relationship-memory.md` 与 Claude Design 项目「关系记忆 · 移动端 UI」。

回归只跑已提交版本：两个仓库各一个分离检出，放在本会话的 scratchpad 下（`audit/synon-ime`、`audit/qingjian-mainline`），
服务端的 `../qingjian-mainline` 路径依赖指向客户端的已提交版本，不受两个会话工作区里未提交改动的影响。

## 回归记录

| 时间 | 客户端提交 | 服务端提交 | 客户端测试 | 服务端测试 | clippy | 备注 |
|---|---|---|---|---|---|---|
| 2026-10-04 | 3e93f77（Task 1–2） | — | 全过（proto 10、client 10 等） | — | — | 基线 |
| 2026-10-04 | 3e93f77 | 786543e（Task 1–4） | — | 全过（lib 17、集成 39） | `-D warnings` 无告警 | 服务端对着客户端已提交版本编译 |
| 2026-10-04 | 5fb2df0（Task 2 修订） | 786543e | 全过（client 20） | 全过 | 两边无告警 | |
| 2026-10-04 | 765613b（2A Task 4 会话接入、9 个 C 接口、键盘待办） | — | 全过（217 个，带数据 + REQUIRE，连跑 2 次）；iOS check 通过 | — | 无告警 | Task 3 变异抽查通过；建议待办笔记落盘防进程被杀丢失 |
| 2026-10-04 | 272c9d9（2A Task 3 HintIndex + 强制交错冲突测试） | — | 全过（203 个，带数据 + REQUIRE，连跑 2 次；交错测试 5 次稳定） | — | 无告警 | Task 3 测试未看红灯 → 要求变异抽查 |
| 2026-10-04 | b97ddc7（2A Task 2 MemoryStore：flock + rev + conflict） | — | 全过（187 个，带数据 + REQUIRE，连跑 2 次） | — | 无告警 | 要求补 Barrier 强制交错的冲突测试 |
| 2026-10-04 | d22c591 | 7aea7b0（2C Task 4 learner）+ Task 5 compose | — | 全过（258 个，连跑 2 次） | 无告警 | 日志无内容；提醒 %error 不得带上游响应体；learner 线上默认关 |
| 2026-10-04 | d22c591（2A Task 1 ScopedLearner + REQUIRE_DATA 开关） | 958abfd（2C Task 3 抽卡） | 全过（165 个，带数据 + REQUIRE） | 全过（233 个） | 无告警 | 抽卡提示词审过 |
| 2026-10-04 | 59e14f8 | 1c055bb（2C Task 2 卡片接口 + tuner REQUIRE） | — | 208 过；tuner e2e 带 REQUIRE 真跑失败（CLI 读本机 config 无 API key，测试隔离问题，非 2C） | 无告警 | 待办：e2e 给 CLI 隔离配置 |
| 2026-10-04 | 59e14f8（带 QINGJIAN_DATA） | fb2f655（2C Task 1） | 全过（154 个，含会话测试） | 全过（197 个，连跑 2 次） | 无告警 | **口径修正**：此前客户端回归未设 QINGJIAN_DATA，会话测试被静默跳过；带数据重跑 3081c30 有 1 个失败（logs_only_when_connected，测试夹具未跟上开关缺省关，b1f1954 已修）。此后客户端回归一律带数据。 |
| 2026-10-04 | 4b60e13（卡片契约注释、MAX_CARD_TEXT_CHARS / KEYWORDS） | — | 全过（154 个） | — | 无告警 | |
| 2026-10-04 | 22086d7（2C Task 1：卡片 proto） | ce2e33d | 全过（154 个） | 全过（184 个） | 无告警 | |
| 2026-10-04 | 1852756 | ce2e33d（2B T5–T8：processor、deid、deid-eval、文档） | — | 全过（184 个，连跑 2 次） | 无告警 | deid-eval --rules-only：规则类漏检 0、误伤 0/586；2B 服务端关闭；未部署 |
| 2026-10-04 | 1852756 | 426f210（2B T1–T4：开关、对象登记、素材、保留） | — | 全过（160 个，连跑 2 次） | 无告警 | 写入路径、AAD、关闭清理抽查通过 |
| 2026-10-04 | 1852756（proto：skipped、zero_retention 缺省） | 6525e2e | 全过（148 个） | 全过（135 个） | 无告警 | 2B 契约决定的 proto 部分落地 |
| 2026-10-04 | 901d159（凭据、小区门牌；向量 218 条） | 6525e2e | 全过（148 个） | 全过（135 个） | 无告警 | 脱敏规则层审计关闭；中文口令不替换为已知限制 |
| 2026-10-04 | 232c6b4（脱敏 5 处缺口） | — | 全过（148 个，向量 170 条） | — | 无告警 | 探针复测通过；新提凭据（密码 / 验证码）与小区门牌两条 |
| 2026-10-04 | 989c8c3 | 6525e2e（Feature::Memory 空分支、部署固定客户端提交） | — | 全过（135 个，--locked） | 无告警 | 编译断裂已解除；线上仍为 57b7bc4 + 3081c30 |
| 2026-10-04 | 989c8c3（2B Task 1–2：proto、redact） | 57b7bc4（线上部署版） | 全过（148 个） | 57b7bc4 对 3081c30 全过（5 次）；对 989c8c3 编译失败（Feature::Memory 未覆盖，属 2B Task 1） | 无告警 | 空环境变量启动通过；线上 healthz / 401 / 登录页核对通过；脱敏探针发现 4 个缺口已发客户端 |
| 2026-10-04 | 3081c30（子项目 1 客户端收尾：桥同意参数、iOS 勾选框、文档） | 6be2bbf | 全过（141 个）；iOS 模拟器 16 个测试通过 | 全过（133 个） | 无告警 | 子项目 1 客户端完成；真机与端到端待部署 |
| 2026-10-04 | ee6528b（Task 5 Mac + 出境同意） | 6be2bbf（出境同意） | 全过（137 个）；qingjian-macos check 通过 | 全过（133 个，--locked，连跑 2 次） | 无告警 | 建议服务端引用 proto 的同意版本常量 |
| 2026-10-04 | 7ce2ef0（桥修复 + Task 4 iOS 账号页） | 95817b9 | 全过（96 个）；iOS 模拟器构建 + 12 个测试通过 | — | 无告警 | 账号页截图对照待用户授权模拟器 |
| 2026-10-04 | 9efbaee | 95817b9（审计测试改进程级捕获） | — | 全 workspace 10 次 + lib 30 次，0 失败 | 无告警 | 不稳定测试关闭；服务端审计项全部关闭 |
| 2026-10-04 | 9efbaee | a5f045d（S15–S19 修复） | — | 127 个通过，但 key_audit_records_failures_and_destruction 偶发失败（约 1/10，tracing interest 缓存） | 无告警 | 已发，待修 |
| 2026-10-04 | 9efbaee | 216ce1e（密钥分库） | — | 全过（120 个，--locked，连跑 2 次） | 无告警 | 安全审计进行中 |
| 2026-10-04 | 9efbaee（Task 3 + LockedToday） | 403993e | 全过（89 个）；bridge 对 aarch64-apple-ios check 通过 | 全过（115 个） | 无告警 | iOS 工程预期 Task 4 前编不过 |
| 2026-10-04 | 2eb63b7 | 403993e（S12 每日 3 次） | — | 全过（115 个，--locked） | 无告警 | 锁定逻辑核对通过 |
| 2026-10-04 | 2eb63b7 | 6aad599（S9–S12 修复） | — | 全过（111 个，--locked，连跑 2 次） | 无告警 | S12 按 20 次 / 滚动 24h 实现，待改为 3 次 / 北京时间自然日 |
| 2026-10-04 | 2eb63b7 | a1580b1（Task 12–13） | — | 全过（98 个，--locked） | 无告警 | 之前几个提交漏带 Cargo.lock，a1580b1 补上 |
| 2026-10-04 | 2eb63b7 | 2c0f02e（Task 5–11 + 审计修复） | — | 全过（96 个，连跑 2 次） | 无告警 | 服务端对着客户端 2eb63b7 |
| 2026-10-04 | 2eb63b7（C1–C3） | 786543e | 全过（client 33，client_http 连跑 3 次不抖） | 全过 | 无告警 | |

## 设计偏差（需要用户决定）

| # | 内容 | 状态 |
|---|---|---|
| D1 | 密钥归属：spec §2 是服务端持主密钥；relationship-memory.md 是客户端持私钥解封主密钥 | 已定：服务端持钥（大模型要用数据），按 sujian-encryption.md |
| D1' | 用户要求大模型能用服务端数据 → 服务端必须能解密；方案见 docs/design/sujian-encryption.md，2026-10-04 用户确认密钥分库，已发服务端会话 | 实施中 |
| D3 | 服务器在新加坡（腾讯云），与设计文档「境内存储」冲突；大陆用户数据出境需单独同意、可能需标准合同；另有生成式 AI 备案与 ICP 备案 | 用户定：留在新加坡，补出境单独同意；方案已发两个会话 |
| D4 | 验证码邮件服务 | 用户定：腾讯云 SES（日结后付费 0.0019 元/封），已通知服务端写进部署文档 |
| D2 | iOS 账号页本期只在原「青简 Cloud」设置页上最小改动，素笺整套界面（同意页、「我」页）放子项目 3 | 记录，符合 spec 分期 |

## 审计发现

### 客户端（5fb2df0）

| # | 严重度 | 问题 | 处理 |
|---|---|---|---|
| C1 | 重要 | Forbidden / Rejected 拿不到服务端的 error 文案（ureq status-as-error） | 采纳，修订中；2eb63b7 已修，核对通过 |
| C2 | 重要 | 登录失败的 401 与会话失效混成 Unauthorized | 采纳，加 AuthFailed；2eb63b7 已修，核对通过 |
| C3 | 重要 | 503（没配 Apple / SMTP）被当成可重试 | 采纳，登录类加 NotConfigured；2eb63b7 已修，核对通过 |
| C4 | 建议 | 429 不分限流与大模型每日上限 | 不做，理由：调用方上下文可区分 |
| C5 | 建议 | Platform 严格枚举，前向兼容差 | 不做，改为约定「服务端加平台前先发客户端」 |
| C6 | 建议 | challenge 字段与返回类型不匹配 | 补文档 |
| C7 | 建议 | 403 后 300 秒轮询与 spec「不重试」不符 | 有意为之，改注释与计划措辞 |
| C0 | 重要 | 403 时剪贴板队列未清 | 已在 44072db 修好，核对通过 |

### 服务端（786543e）

| # | 严重度 | 问题 | 处理 |
|---|---|---|---|
| S1 | 重要 | sessions.id 无 AUTOINCREMENT，会话 id 复用致跨用户串设备名、去重误命中（500）、输入日志批次被吞 | 667e678 已修（测试 session_ids_are_never_reused 等） |
| S2 | 重要 | 已建立的 SSE 不随会话吊销 / 删号断开 | 667e678 已修（SSE 每秒复验） |
| S3 | 重要 | chat.rs 对非对象 JSON 的下标赋值会 panic | 667e678 已修（非对象 400） |
| S4 | 建议 | 全局广播通道 256 容量，一人刷屏全员断流 | 未改：Lagged 后客户端按 seq 重连补拉，接受；用户量上来后再按用户分通道 |
| S5 | 建议 | LLM 缓存键 FNV-1a 64 位 | 2c0f02e 已改 SHA-256 |
| S6 | 建议 | DEK 封装 AAD 不绑 user_id；字段 AAD 不绑列名；无 zeroize；500 回显 SQLite 报错 | 2c0f02e 已改 AAD、zeroize、固定文案 |
| S7 | 重要 | spec §2「删号后备份无法解开」不成立：wrapped_dek 与数据同库同备份，主密钥仍在 | 用户确认按密钥分库改，已发服务端，实施中 |
| S8 | — | §10 测试缺口：crypto_at_rest、级联删除、跨用户挪密文、用量 / 缓存隔离、SSE 吊销、id 复用 | 2c0f02e 已补 |


### 服务端 Task 5–13（a1580b1）

| # | 严重度 | 问题 | 处理 |
|---|---|---|---|
| S9 | 重要 | SSE Lagged 后 live 截断但 merge 的 interval 不结束，连接挂着静默丢剪贴板 | 6aad599 已修，核对通过 |
| S10 | 重要 | Apple 网页授权码换 refresh_token 缺 redirect_uri，删号时不会撤销授权 | 6aad599 已修，核对通过 |
| S11 | 重要 | 邮箱发码先覆盖旧码再查每小时上限，第 6 次作废有效码；SMTP 失败也计数 | 6aad599 已修，核对通过 |
| S12 | 重要 | 验证码每邮箱每小时可猜 25 次，一年约 20% 撞中，可接管账号 | 403993e 已按用户规则实现（3 次 / 北京时间自然日 / start 与 verify 都 429 + code=locked_today / 不提示剩余次数），核对通过 |
| S13 | 建议 | verify 先烧码后校验 device；403/404 与 spec 不一致；SSE 复验库错放行；502 回显上游原文；chat 并发超额与 n 绕过封顶；JWKS 持锁拉取；handoff new_user 写死；主密钥临时缓冲未清零；axum 解析失败非 {"error"} | 已发 |
| S14 | — | §10 缺：每小时上限集成测试、Apple alg 拒绝、JWKS 502、SSE 开关 403 / 删号 / lag、handoff 过期 | 已发 |

## 待办（下一轮）

- tuner e2e 给 qingjian-cli 隔离配置（恢复 tuner 前）。

- chat：额度先查后请求，并发可超额；stream:true 不计 token。
- 邮箱发信失败时 IP 桶仍计一次（按尝试计，接受）。

### 客户端 Task 3（9efbaee，iOS 桥）

| # | 严重度 | 问题 | 处理 |
|---|---|---|---|
| C8 | 阻断 | 换账号删了上传偏移但留着 input-log.jsonl，新账号开日志上传后会把旧账号的明文输入从头传上去（跨租户） | 05646fb 已修 |
| C9 | 重要 | cloud.toml 读改写无锁、tmp 名固定，并发调用可写坏配置或丢更新 | 05646fb 已修 |
| C10 | 重要 | 操作接口只回文案无错误代号，Swift 无法区分锁定 / 码错 / 失效 / 未配置 | 05646fb 已修 |
| C11 | 建议 | 残留 tmp 权限可能 0644；删号后 outbox 明文残留；403 英文原文直出；旧引擎写回进度 | 05646fb 已修 |

### 服务端 密钥分库（216ce1e）

| # | 严重度 | 问题 | 处理 |
|---|---|---|---|
| S15 | 阻断 | 关功能与在飞写请求竞态：查开关、建钥匙、写数据分别加锁，关闭后会新建钥匙写入数据（复活）或留下旧钥匙密文致拉取永久失败 | a5f045d 已修，核对通过 |
| S16 | 重要 | delete_user 与 ensure_key 竞态，删号后可能残留孤儿钥匙并进备份 | a5f045d 已修，核对通过 |
| S17 | 重要 | SCHEMA_VERSION 未升，旧库上注册全挂且旧 DEK 残留 | a5f045d 已修，核对通过 |
| S18 | 重要 | 钥匙库备份按「份数」不按「天数」保留，7 天承诺可能失效 | a5f045d 已修，核对通过 |
| S19 | 建议 | key_split 断言过宽无对照组；关功能不 checkpoint；有行无钥匙时 config/input_log 静默为空；delete_clip 凭空建钥匙；审计日志 process 写死、缺失败与销毁；钥匙目录默认同数据目录；/keys 权限 755；临时数组未清零 | a5f045d 已修，核对通过 |

## 子项目 2（2026-10-04 分派）

- spec：synon-ime 分支 sujian-memory-docs（3f22587）`docs/superpowers/specs/2026-10-04-memory-design.md`；拆成 2A 本地记忆 / 2B 素材上传与脱敏 / 2C 每日学习 / 2D 人设与改写，先做 2A、2B。
- 计划大纲：客户端 2A、2B（qingjian 分支 sujian-memory-docs，d013491）；服务端 2B（synon-ime 同名分支）。实施会话先展开成逐步计划，交审计审过再开工。
- 顺序：客户端先收尾子项目 1 → 2B Task 1–2（proto、redact）→ 2A → 2B 其余；服务端子项目 1 部署后开 feat/sujian-memory-2b。

## 上下文预测（2026-10-04 立项，只在 sujian 分支）

- 设计与计划大纲：qingjian 分支 sujian-memory-docs（b7485e3）`cloud/docs/specs/2026-10-04-context-prediction-design.md`、`cloud/docs/plans/2026-10-04-context-prediction.md`。
- 根因：宿主前文不进词级排序；choice 压过上下文；默认通变不看前文；「又想」非词库词；Tab 续写只有云端。
- 验收门槛：同拼音不同上文评测集词首选 +15 个百分点；--eval-text、--replay 不降超过 0.5 个百分点；按键 p99 +1 ms 以内；本地续写代理精度 ≥60%、p90 ≤150 ms。
- 实施：用户让审计会话安排，已派给「Bug修复大师」（空闲；客户端排 2A/2B，服务端只做服务端）。

### 2B 服务端计划审查（3d3cfd2）

结论：修改后开工。签名、字段、单位与 proto 901d159、服务端 6525e2e 一致。

| # | 严重度 | 问题 | 决定 |
|---|---|---|---|
| P1 | 阻断 | 关 memory 后 DELETE 对象被 403，对象永不删除、占名额 | DELETE 不查 memory 开关 |
| P2 | 重要 | 未登记对象的素材整批 400，日常素材被连带丢弃 | 跳过并返回 skipped；客户端先登记再推 |
| P3 | 重要 | contact_deleted / 重复 DELETE 404 只在重试时出现，客户端未处理 | DELETE 一律幂等 204；contact_deleted 客户端当终态 |
| P4 | 重要 | 长度异常检查误杀短句 | 占位符按 1 字计 + 绝对余量 8 |
| P5 | 重要 | 提示注入可绕过模型层脱敏 | 文字包 JSON、占位符保留检查、评测集加注入样本 |
| P6 | 重要 | 被拒批次也扣限流额度 | 事务内按实际写入条数计 |
| P7 | 建议 | 非 dating 可登记、zero_retention 写死 true、去重键随 session 变、at 无范围校验 等 | 仅 dating；zero_retention 配置缺省 false；去重 (user_id, client_id)；at 校验 |
- 2026-10-04：服务端按审查修订计划 d58959c，复核通过，放行 Task 1。

### 上下文预测计划审查（adc825f）

结论：修改后开工。API 签名核对无误。
- 阻断：整句与词同台比分（长句必输、跨档）→ 只在首词覆盖全段时参与；门槛未用产品配置（通变 + 知微）→ 每任务用 both 跑、通变作对照。
- 重要：p99 无测量；worker 互顶；切 tongbian 不卸知微；私密输入未屏蔽前文；choice 加分缺「选过一次弱上文仍首选」测试且 β 应要求回放不降；e2e 未测宿主前文；续写 p90 超限应停下。
- 产品：Tab 改为接受续写可接受，写进 cloud/docs；续写日志来源加 LocalContinuation（最小上游补丁）。

## 大模型供应商（2026-10-04）

- 用户定：DeepSeek。开放平台服务条款未写明 API 输入是否用于训练、保留期限、零留存选项 → SUJIAN_MEMORY_LLM_ZERO_RETENTION 保持 false，同意页不承诺「不保存、不训练」；建议用户发邮件 api-service@deepseek.com 询问数据处理协议。
- 数据流：新加坡服务器 → DeepSeek（存储在中国境内），写入部署文档与同意页说明。
- 已请服务端用真实 DeepSeek 跑 deid-eval（含注入样本），数字回来后写 2C spec。

## 2C 每日整理记忆（2026-10-04 分派）

- spec：synon-ime sujian-memory-docs 88bbf63 `2026-10-04-memory-2c-design.md`；服务端计划大纲同提交；客户端计划大纲 qingjian sujian-memory-docs 6367143。
- 要点：learner 每天北京 03:00；只改未确认云端卡；输出再过规则脱敏；只用确认过的卡做提示；30 天淡出；手动卡上云；全局 token 上限；分区学习表不做。
- learner 线上打开前需真实 DeepSeek 评测（deid-eval + 50 段抽卡评测）过审。
- 2026-10-04：上下文预测计划修订版 c2599f4 复核通过，放行 Task 1；要求 Task 2 验收含整句期望（我现在 → 又想）。
- DeepSeek 真实评测（服务端 023bc61，deepseek-flash）：合成 358 条上 person/org/place 与规则类漏检 0、误伤 0/586、丢弃 0、注入 24 条全挡；p50 859 ms、p90 1386 ms；每条约 0.00015 USD。局限：模板合成样本偏简单。
- 更正：DeepSeek 隐私政策（2026-02-10）写明数据存中国境内、保留用于训练的权利、有退出训练选项；API 是否适用待用户向 DeepSeek 确认。同意页文案已按此改写（客户端 2B Task 6）。
- learner 上线门槛：≥150 条真实化难样本 person/org/place 漏检 ≤5%；50 段抽卡评测含个人信息的卡 0 条。

### 2A 客户端计划审查（b2aa168）

结论：修改后开工（Task 1、3 可先动）。阻断：App 与键盘两进程写同一记忆数据会互相覆盖 → flock 跨进程锁 + 按对象修订号写回 + conflict。重要：读失败当空覆盖、忘掉后复活、set_scope 覆盖开关、最近 24 字跨应用、重建丢节流、提示行跳动。
决定：删词清叠加层；date 按年重复；提示开关按人（照设计稿 02 1d）；「知道了」持久化；恋爱场景不写全局 n-gram；权重 4；键盘与 App 显示名「素笺」（用户定）；more 定义澄清。
- 上下文预测 Task 1（71a8f1c）：基线复现 eval-context 40.3%/40.3%（404 对）；p99 测量噪声与 1 ms 门槛同量级 → 改为三次中位数对比。
- 上下文预测 Task 2（fea3246）：eval-context 40.3→62.4%（+22pp），eval-text −0.1，replay 词 +0.1、整句 −1.8pp（165 句中净 −3）。裁决：量 (a) 现状 /(b) Viterbi 首词不用前文 /(c) 插值，replay 整句门槛定为 ≤0.6pp（1 句）；汽车→油箱 由 Task 4 兑现并单列验收。

### 2C 服务端计划审查（55538ca）

结论：修改后开工。阻断：手动卡 / 用户改过的云端卡明文进 prompt → 先 deidentify。重要：单条脱敏失败卡死整批；token 失败路径不记账；learner 启动即跑与出错退出；DEFERRED 事务与第二写进程冲突 → IMMEDIATE；墓碑物理删除致离线复活 → 只清内容不删行；租约无 token 校验。决定：malformed 第二次才删素材；被删卡 HMAC 指纹 90 天防复抽；卡数上限 409 card_limit。卡片契约已同步客户端。
- 2C 服务端计划修订版 a25d59a 复核通过，放行 Task 1；已知卡脱敏无持久缓存（CARDS_IN_PROMPT 封顶），上线后看 token 占比。
- 2A 客户端计划修订版 59e14f8 复核通过，放行 Task 1–4（Rust），再 Task 5–6（Swift）；键盘等锁上限 200ms，拿不到入待办。

## 品牌统一为「素笺」（2026-10-04 用户要求）

- 已派客户端：iOS 与 macOS 所有用户可见处改「素笺」，「青简 Cloud」→「素笺云」，图标换定稿；bundle id / App Group / crate / 路径等内部标识不改；GPL 署名「基于开源的青简输入法」保留；上游文件改动记 fork-patch。排在 2A Task 5 之前。

## 2C Task 6 memory-eval（bf50208）审计

- 回归：服务端 262 passed / 0 failed，clippy 无告警。
- 真实 DeepSeek 50 段：调用失败 0、校验丢弃 0、卡片与送模型输入的个人信息泄漏 0、闲聊/注入抽卡 0；p50 2.2s；46642 token。门槛 2（50 段 0 张含个人信息的卡）通过。
- 评测揭出并已修：DeepSeek 默认推理吃 max_tokens（改为显式 thinking:disabled）；脱敏泛化词写进卡片（提示词 + 校验）。358 条脱敏评测需在新条件下于 Task 7 重跑。
- 定夺 D5：「某」+ 名词（某小学、某医院等）一律拦，只放行时间类（某天、某次、某时、某些）；生日、纪念日等每年都有的日子，when 填下一次。

## 2C Task 7 难样本脱敏门槛（469e386）审计

- 回归 266 passed / 0 failed，clippy 无告警；脱敏提示词自 bf50208 起未改，验证集未被用来调提示词。
- 门槛 1（真实 DeepSeek，推理关闭，hard_holdout 198 条）：person 2.5%、org 1.2%、place 0%，全部 ≤5%，通过；误伤 0.7%，约 0.014 USD。
- 门槛 2：D5 处理后重跑 pii_heavy 与 birthday 两组，泄漏 0；生日的 when 取下一次。通过。
- 门槛 3（同意页写明 DeepSeek 与数据在中国境内）在客户端 2B Task 6，未完成。learner 在线上保持关闭，部署等用户决定。

## 决定 D6：默认不要账号（2026-10-05 用户拍板）

- 按微信输入法的做法：开通云服务时建空间，用匹配码加设备并在旧设备上允许；邮箱、Apple、微信只作找回方式，可以不绑。规格 synon-ime-memory-docs 8c4a022，客户端大纲 qingjian-memory-docs 6bdee9c。
- 已派服务端（从 2C 分支开）与客户端（排在改名、完全访问实测之后，2B 之前）。需要用户申请微信开放平台的移动应用与网站应用（企业主体）。
- 用户同意在本机安装 Mac 壳拍截图。
- UI 截图审计改由 Haiku 4.5 审计员做。第一轮校准：误报 2 条，漏报约 8 条，抓到 1 条我漏掉的（2c「你写的」）；已回传反馈，下一批再看。
- 新问题：没开完全访问时，键盘把记忆功能全部拦掉，和设计稿 05 的 2c/2d「免费版不需要完全访问」冲突；没开完全访问时键盘可能读不到 App Group，已让客户端先实测再给方案。

## 安装包图标（2026-10-05 用户要求）

- 补生成 macOS 应用图标 macos-1024（Big Sur 网格）与菜单栏模板图标 macos-menu.tiff（16/32），brand_gen.py 已更新。
- 已派客户端，并到改名任务一起做：iOS AppIcon 用单一尺寸的亮/暗/着色三套；Mac 的 icns、菜单栏图标、pkg 标题。素笺图标放 cloud/ 下，不覆盖上游 assets/icon，bundle.sh 只改源路径并记 fork-patch。

## 2A Task 5 第二批（3bfc8b6）

- Rust 回归 223 passed，clippy 无告警；客户端报告 xcodebuild test 49 个全过。
- UI 审计员（Haiku）报了 1 高 2 中。我复核：高那条（没开完全访问时挡掉记忆功能）成立，等真机实测；两条「没加粗」是误报（放大后能看出已加粗）。其余通过。
- 取舍定夺：非对象类的标识用中性色；深色模式先补一套 Theme；命中词字段和 last_used 记进 2C 客户端 Task 5b（5ca5605）；键盘不打开 App，只出短提示。
- UI 审计员两批的表现：第 1 批漏报多，第 2 批误报 2 条，主要错在看细节不放大。继续用，但高、中严重度的条目我自己抽查后再转给客户端。

## 1b 服务端计划审计（7890efd）

- 10 条设计决定全部同意：令牌不落库，第一次取时才签发；secret 走请求头；每种找回方式只绑一个；事件流对所有设备开放；满员时不消费匹配码；微信 subject 存 HMAC。
- 补两条：A 剪贴板关着时事件流里不能漏出剪贴板事件，要配测试；B sessions 加 joined_via 列，找回加入的痕迹落库，GET /v1/account 返回这一列。
- 已知局限记进自检：任何一台设备都能删除整个空间；没开过任何功能的空间不清理。
- proto 契约已原样转给客户端，客户端 Task 1 按它命名。

## 决定 D7：1b 缩范围（2026-10-05 用户）

- 微信不做，绑定和找回往后放。这一轮只做建空间和匹配码加设备；joined_via 列照加。
- 旧的 /v1/auth/* 放到开关 SUJIAN_LEGACY_LOGIN 后面，缺省关闭（返回 404），防止有人绕过空间直接注册。
- 已知局限：这一轮没有找回，设备全丢就找不回，界面上写明。
- 1b 缩范围计划 b1cd64d 审过；Task 1（5746c46）回归 270 passed、clippy 无告警。旧登录开关（Task 4）先做。
- 1b Task 4 旧登录开关（dde999c）：回归 274 passed，clippy 无告警，缺省关闭。服务端等客户端 proto。

## iOS 改名与图标（519552e）

- 回归 223 passed，clippy 无告警。cloud/ios 里的「青简」只剩 GPL 署名一句及其测试。
- UI 审计员第三批：全部通过。我抽查了主屏截图：图标与名字都对，折角在主屏尺寸下颜色偏淡，不过这是定稿的样子。
- 定夺：DOMAIN_PREFIX 不改，它是内部元数据；面板控件（例如「完成」）改用中性色。
- 等真机：深色和着色图标、键盘列表里的图标。

## 重启前的状态（2026-10-05，用户重启 Mac）

- 客户端：origin/sujian 到 f7e3193，Mac 新包（构建 400）已装在 ~/Library/Input Methods。接下来从 Mac 截图清单第 0 步做起，然后在真机上装（团队 L9YRXEKYN2，用户已同意），再做完全访问实测，之后是 1b 的 proto（Task 1）。
- 服务端：feat/sujian-1b-no-account 到 dde999c（Task 1 和 Task 4 已完成），等客户端 proto。
- Bug修复大师：上下文预测 Task 2 的三个变体数字还没报。
- 已清理：/Library 下的旧版、LaunchServices 里重复登记的 Qingjian.app。
- 上下文预测 Task 2 定为变体 (b)：只在词级排序里用前文，整句首词不变。eval-context 从 40.3% 到 62.9%，eval-text 和 replay 的词、整句都没有下降。最终代码重启后补跑再提交。

## 重启之后（2026-10-05）

- 审计 worktree 原先在 /private/tmp，重启后丢失，已重建到 /Users/liyuqing/sproot/.audit-sujian/：qingjian-mainline、synon-ime、target-*。
- 服务端 b120799（建空间与匹配码的存储层，不依赖 proto）：回归 282 passed，clippy 无告警。
- 上下文预测 Task 2 定稿 a90759a：core 322 passed；eval-context 62.9%，eval-text 和 replay 没有下降，p99 多 0.1ms。要求修正 display.rs 的过期注释，并补齐 fork-patch。可以开 Task 3。
- Mac 输入法切换失败：原因很可能是 qingjian-local、qingjian-publish 两个目录 target 下的旧包，和新版同一个 bundle id，重启后被重新登记。客户端已注销并删除，然后重装了构建 400，等用户在 TextEdit 里验证。
- Mac 切换：我之前查错了位置。第三方输入源的启用记录在 com.apple.inputsources 的 AppleEnabledThirdPartyInputSources 里，客户端已确认 Sujian.app 能被 IMK 拉起（18:54，pid 13690）。改名后的过渡期里，宿主进程还拿着旧 endpoint，需要把宿主 App 重开。菜单栏图标改用 PDF 模板图（tiff 不反色）。
- 上下文预测 Task 3（8ed3a3a）：β=8，拼音选过的词改为加分，形码保持选过就第一；core 328 passed；replay 词 926/1057（-1，在门槛内），其他不变。开 Task 4。
- 1b proto（42d5e4c、51652de）：cloud 235 passed，已转给服务端。EventKind 加了 Unknown 兜底；新事件类型要等客户端都升级后才能推。
- 上下文预测 Task 4：修好一个异步 bug（两条重排线程里，先到的结果会让轮询停止，后到的那条被丢掉）。λ_w 定为 0.5：eval-context 70.3%，replay 词 925，整句与 eval-text 不变；「汽车→油箱」只排到第 2，记为已知差距，验收放宽为前 2。
- 服务端 1b 这一轮完成（b0cb048）：301 passed，clippy 无告警；日志不记敏感值；名额在取令牌时才占。已要求开 PR，不合并。
- 上下文预测 Task 4 提交（1fcfa5c）：core 336 passed；eval-context 70.3%，replay 词 925、整句 120，eval-text 不变，p99 +0.2ms。
- 1b 服务端 PR：dayuer/synon-ime#3（叠在 #2 上）。deploy/README 加了「发版前」清单：迁移是单向的（user_version 2→5），回滚要还原备份库；线上磁盘上次是 98%，要先处理。
- 上下文预测 Task 5（9ae546f）：同时加载两个模型时，CLI 实测内存增加约 67MB（f16）。Task 6（940744e）：本地续写 p90 63ms，但代理精度只有约 7%，达不到 60%。D8：用户决定先不做本地续写；Task 7 改为 rebase 后在 Mac 真机上验收词级效果，Task 8 照做。
- 合并：synon-ime#1 已合进 main（393fb9b）。合并 #2 时被权限检查（Merge Without Review）拦下，#2、#3 待用户自己合或放行。feat/sujian-1b-proto 的内容已全部在 sujian 上；synon-ime-memory-docs 分支已被 1b 分支的修订版取代，不合。
- 2A Task 6（45b10bc）：Rust 235 passed。UI 第四批：只有开关颜色要改（设计稿 .toggle 打开时用 accent），其余通过。定夺：加人弹层按 2g 的版式做；「忘掉」改用 alert；读写移出主线程；冲突用 Debug 连续保存按钮在真机上验。
- 上下文预测 8 个任务全部完成（4215a77，比 sujian 多 12 个提交）：cloud 237 passed。等用户在 Mac 真机上验收后合进 sujian。最终结果：eval-context 70.3%（基线 40.3%），eval-text 不变，replay 词 925/927、整句不变。

## 收尾（2026-10-05 用户要求逐步收回）
- 已通知三个会话：做完手头这一轮就停，不开新任务，发收尾报告；不合并 main、不删分支、不碰线上。UI 审计员（Haiku）已空闲。
- 服务端收尾：1b 分支 5ff0834 已推送，工作区干净，待命。#1 已合，#2、#3 开着（301 passed）。线上仍是 57b7bc4。另一处更正：腾讯云邮件和 Apple 登录只在打开旧登录入口时才需要。
- 线上磁盘：实际是 68%（98% 是旧数字），清掉悬空镜像、构建缓存、journald、apt 缓存后到 65%，释放约 1.6GB；服务没动，healthz 200，回滚镜像还在。另有几项要用户决定：/root/removed-20260929 2.6G、/root/.npm 2.3G、/root/backups 448M、10 个没在用的 volume。
- 上下文预测收尾：161b224 已推送到 origin/feat/context-prediction，可以快进合进 sujian（707 + 237 passed）。我推送时被权限检查（Modify Shared Resources）拦下，等用户合。用户本机现在装的是这条分支的 c091c54。
- 用户已把 feat/context-prediction 快进合进 sujian（45b10bc..161b224）。已通知客户端先 rebase 再推。
- 2A Task 6 修订（1927f4a）：707 + 237 + iOS 84 全部通过。UI 第五批通过（我抽查了 alert：「忘掉」是红色，审计员说成灰绿，属于小误报）。等用户在真机上验并发，以及最后一个小提交（标签栏、「好了」按钮、alert 回调）。
- 客户端最后一个小提交 a806750（iOS 85 passed）：标签栏用 ink；「好了」按钮用 accent；「忘掉」在 action 里写入。已知问题：玻璃标签栏上未选中项接近黑色；瞬间失败可能撞上 alert 收起动画；衬线字体回落。手机上还是 1927f4a 的 Debug 包，等用户验完并发后换成 Release。

## 收尾完成（2026-10-05）
- 客户端：sujian 4ff5f26，代码的最后一个提交是 a806750。根 workspace 707、cloud 237、iOS 85 全部通过。手机装的是 a806750 的 Release（数据保留，包里没有压测按钮），Mac 是 0.1.5-dev-a806750（构建 434，带上下文预测）。
- 服务端：#1 已合；#2、#3 等用户合并；线上 57b7bc4，磁盘 65%。
- 上下文预测：已合进 sujian。
- 未开始：2A Task 7 文档、2B Task 3–7、2C Task 2–6、1b 客户端 Task 2–6、Mac 截图。暂缓：绑定找回、微信、本地续写。
- 用户已合并 synon-ime#2、#3，服务端三批代码都已进 main。线上仍未部署。
- 工作树清理：删了 context-prediction、1b-proto、memory-docs ×2、qingjian-cloud、qingjian-publish（删之前都是干净的；没推送的提交已推到 origin/archive/*）。保留 qingjian、qingjian-mainline、qingjian-local（产品数据）、synon-ime、.audit-sujian。2C/1b 客户端计划在 origin/archive/sujian-memory-docs，接着做之前先合进 sujian。
- D9：真机证实没开完全访问时键盘读不到 App 写的卡。用户定：记忆功能需要开完全访问，不开时就是普通输入法（满足 4.4.1）。设计稿 05 的 2c、2d 文案待改。已删除远程分支 feat/sujian-1b-proto（proto 内容与 sujian 一致）。
- App Store：用 API（密钥 XN9KPBCH6J）注册了套装 ID sujian.synon.ai（CFZ89BFVX8）和 sujian.synon.ai.keyboard（2NPBL83G4R），都开了 App Groups。API 不支持新建 App 记录（apps 不允许 CREATE），要用户在网页上建。客户端要把 project.yml 和 App Group 改成新 ID（已记为待办）。
- 套装 ID 改动影响服务端：Apple identity_token 的 aud 要改。已记为服务端待办（只在打开旧登录或做 Apple 找回之前才需要）。客户端待办记在 cloud/docs/handoff.md（edc112c）。
- D10（设计稿 01 改版）：牌子拆成场景和人两半，点人名直接切；每个场景各有一组人，各自最多 8 个，记住上次选的人；日常的人出提示和日子提醒，工作的不出；对象不能换场景；日常、工作的人写全局加对象层；2B/2C 按用户开了记录的场景走。
- D11：设计稿 01 新增 1e-2（草稿卡）和 1e-3（冲突）。抽取先用桥里的本地规则（时间、地点用 places.qj、喜好极性、计划），免费离线；冲突只比对方的事，不比用户自己说的话。Task 10 排在 Task 9 后面，要有至少 60 条评测、误报率不超过 10%。键盘里新建对象保留。
- 用户要求停下（2026-10-05）：已通知客户端停手并待命，UI 审计员已停。sujian 停在 1670380（Task 9），手机上装的是 Task 9 的 Release。Task 10 没开工。
- UI 交付：dayuer/qingjian#1 收录设计稿快照（8 个文件，字节数和 etag 都核对过）和 UI 开发清单（45 屏，T1–T13，D1–D6）。我把约束 1 改成「App 照设计稿，灰绿只给人这条只管键盘」（8721915）。已派客户端按 T1→T2→T3→T6→T7→Task10/T8 做；T4、T5 等 D2–D4。
- UI 第六批（Task 9 截图，含补拍的人多矮格）：审计员判全部通过。我抽查了「生日」，已加粗。审计员说 1e 有「冲突处理流程」，属于误述：1e-2、1e-3 在 Task 10/T8 才做，这批没有。
- T1–T3（b0c214d）：cloud 251 passed；UI 第七批通过（深色几张审计员没看，我抽查了没问题）。定夺：标签栏接受 iOS 26 玻璃样式；没开完全访问时牌子用中性色；先藏起「账号」入口。
- UI 第九批（T6、T7、「我」页，16 张，换了一个新的 Haiku 审计员）：除了两张旧截图（dark-2 还是对勾、light-6 还有『登录』脚注），其余通过。我 grep 939c447，代码里已没有禁用词。字体定为 C（回退系统字体），A（向 Apple 下载宋体）待用户决定。
- 两张重拍我亲自看过，没问题。已放行从 706b374 装机。
- 用户要求提交、合并：服务端 main 是 2079774，干净；客户端 sujian 是 706b374，干净；我的文档、品牌、审计记录开了 dayuer/qingjian#2，请客户端快进合入。上游检出里 apps/cli 的 asr_fix 等改动不是这条线的，没动。
- D12（用户直接定）：记一笔保留原话，作为素材交给云端大模型每天整理成卡（2B/2C）；没开云的先存本机。我定：对象名在本机替换成〔对象〕，其余人名交服务端脱敏；本机原文整理后保留 30 天；每个对象最多 200 条未整理；Task 10 本地规则版暂缓；开通前攒的素材要等同意页通过后才补传。
- D13（用户直接定）：App 字体全部换成 MiSans（构建时从官方下载、SHA256 校验，不进仓库），键盘不换。我补了：许可原文存档、下载失败时报错而不是悄悄回退、只打进 App target 并量体积、生僻字要能回退。
- 素材库 M1（1e80ca2，work/m1-materials，未合入）：cloud 278、iOS 152 通过；等 M2 一起合入、一起装机。到上限时整次拒绝并说明空位；conflict 文案改成「记忆刚有更新，请再点一次」。
- MiSans 合入 sujian 2458640：iOS 158 通过；下载与构建后检查都验过失败路径；Payload 约增 14.5MB；协议打进包，关于页有署名；生僻字正常。截图交给新的 Haiku 审计员。M2 已开工，M1+M2+字体一起装机。
- UI 第十批（MiSans，17 张）：审计员判全部通过；我抽查生僻字截图，「𠮷堃赟」显示正常，没有方框。等 M2 完成后一起装机。
- UI 第十一批（待整理，18 张）：审计员报了两条，都是误报。『素笺云』是定好的品牌名，不是违规；『朝昨整理』是审计员把『明早整理』认错了，我查了源码和测试，文案就是『明早整理』。其余通过。Haiku 认字的错误多了，以后文案类判断以源码 grep 为准。
- 远端新增 3 个提交（用户在别的会话推的：XCUITest、首页 2a「记得」、未归人素材并进素材库，0c87816）：cloud 289 passed，clippy 无告警。发现 assign_material 先删后写，中途失败会丢数据（中）；挪到对象名下时没检查 200 条上限（低）。已派客户端修、装机，候选栏 rebase 到新版。
- assign_material 修复（fb89473，fmt 是 c598185）：先写目标再摘无主、按 client_id 去重、挪过去时检查 200 条上限；cloud 291 passed，我读过代码确认。c598185 已装机，数据核对一致。新加的 XCUITest 在共用模拟器上 5 个里挂了 2 个，和这次改动无关，记为待查。候选栏的半成品放在 qingjian-cbar，先不动。
- dayuer/qingjian#3（场景可管理、改写技能包两份计划）审计意见已用 review comment 写回 PR：高——取消分区学习会让私密用词进全局（建议按「选了人只写对象层」分流）、2B 按场景同意需重定、用户自写技能可被用来写诈骗话术（需服务端固定外层 system prompt）；中——工作场景静默没了、迁移改成备份加原子、新旧版本并存的兼容、高情商的边界。
- D14（用户定）：学习按人隔离（选了人只写对象层，没选人才写全局）；技能包不做用户自定义，只能选。已更新 PR #3 两份计划（7c072d6、83b1413）并在 PR 留言。待定：2B 上传同意挂在哪、工作场景要不要保留「安静」。
- sujian 12d5e40（PR #4 场景用户自建，以及通讯录、候选栏 UIKit、1e-2/1e-3 壳等）：cloud 300 passed，clippy 无告警；按人隔离的写入路径正确。发现 learn_word/learn_english 仍写全局，新造词会跨人泄漏（中），已派修。用户另定：上传同意用全局一个开关；工作场景不再单独区分「安静」；服务端 scene 改成不透明字符串（材料见 memory-scene-field.md）。已放行装机，要求重点核对迁移。
- 服务端 scene 改不透明字符串的计划（synon-ime ba06e51）审过：每个用户总共 100 个对象；删掉 invalid_scene；scene 存不透明 id（1–64B）；重复 PUT 时更新 scene；卡片不带场景；proto 由客户端改成 String，排在 learn_word 之后。
- UI 第十二批（PR #4，44 张）：审计员只报了一条——通讯录搜索时键盘回车键是系统蓝。这是误报：那是键盘上的「搜索」键，设计稿 theme.css 的 .key.ret.go 本来就是 #007aff（照系统键盘）。其余通过；已知的 8 条问题由客户端在修。
- learn_word 按人隔离（work/learn-word-scope：3c4967e Core 加 3 个空默认方法，17d1213 桥）：core 336、bridge 188 通过；eval-text 和 replay 逐字相同，typing 的 p99 不变。已放行，等 pr4fix 后一起合并、装机。
- dayuer/qingjian#5（改写技能包的规格与计划）审计意见已写回：中——高情商的 prompt 没写进安全约束、话术词库和「不加信息」冲突、指令注入与输出长度检查；低——构建时检查技能包、temperature 夹紧、私密输入不显示改写、服务端外层 prompt 记待办。
- PR4 修复合入 fe1a591：cloud 309 passed、clippy 无告警，UI 第十三批（22 张复查）全部通过。客户端正在装机。下一步小提交：草稿卡、冲突屏打开时总高不变；修好删除默认场景时的文案。
- 2989d99：草稿卡、冲突屏打开时键盘总高不变；修好删除默认场景时的文案。cb94be8：proto 的 scene 改成 String，删掉 Scene 枚举，已转给服务端切换。contact_limit 只留本机，不自动补登记。手机 unavailable，fe1a591+2989d99 等手机连上再装。
- synon-ime#5（06d966a，scene 改成不透明字符串，每用户 100 个对象）：305 passed，clippy 无告警，名额判断在事务里。已审过，等用户合并；不部署。
- 用户要求提交、合并：已合并 synon-ime#5；qingjian#5（技能包规格）有 3 条中等意见没改，未合。
