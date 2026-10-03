# 分叉补丁

Cloud 的大部分代码在 `cloud/` 下，与上游隔离。只有学习数据同步非改输入法不可，这部分是分叉补丁。
原则：新代码放新文件，对上游已有文件只加挂钩行，合并上游时冲突面最小。

## 清单

| 文件 | 改动 | 说明 |
|---|---|---|
| `crates/qingjian-core/src/engine/learning/learner.rs` | 加 5 行 | `Learner::merge_remote`，缺省实现什么都不做 |
| `crates/qingjian-learning/src/frequency_learner/remote.rs` | 新文件 | 收件箱解析与合并（含测试） |
| `crates/qingjian-learning/src/frequency_learner/mod.rs` | 加 1 行 | `mod remote;` |
| `crates/qingjian-learning/src/frequency_learner/learner_impl.rs` | 加 4 行 | `FrequencyLearner` 实现 `merge_remote` |
| `apps/macos/src/host/cloud/inbox.rs` | 新文件 | 每拍读 `sync/inbox.tsv`、合并、删文件 |
| `apps/macos/src/host/cloud/mod.rs` | 加 1 行 | `mod inbox;` |
| `apps/macos/src/host/config/mod.rs` | 加 1 行 | `tick()` 里调 `apply_cloud_inbox()` |

### 回车上屏高亮候选（用户要的小功能，只在 macOS 生效）

| 文件 | 改动 | 说明 |
|---|---|---|
| `crates/qingjian-platform/src/config/general.rs` | 加 4 行 | `[general] enter_commits_candidate`，缺省 false |
| `crates/qingjian-platform/src/config/mod.rs` | 加 2 行 | 配置模板里的说明 |
| `apps/macos/src/imk/controller/command.rs` | 加 18 行 | 回车分支按开关选高亮 / 原样；⇧ 从当前键盘状态读 |
| `apps/macos/src/preferences/setting/mod.rs` | 加 6 行 | `Setting::EnterCommitsCandidate`，tag 取 99 避开上游往后顺排的号 |
| `apps/macos/src/preferences/pages/general.rs` | 加 20 行 | 「通用」页勾选框 |
| `apps/macos/src/host/settings.rs` | 加 4 行 | 勾选框写回配置 |
| `docs/user/getting-started/keys.md` | 改 1 行、加 1 段 | 按键说明 |

Windows、Linux 的 Server 不受影响（`merge_remote` 有缺省实现）；以后要接入时各加一个同样的 `inbox` 挂钩。

## 合并上游时

1. 冲突只可能出在上表「加 N 行」的那几个文件，按上游的新写法把挂钩行重新加回去。
2. 上游改了 `FrequencyLearner` 的字段或 `UserNgram` 的文本格式：跑 `cargo test -p qingjian-learning remote`
   与 `cd cloud && cargo test --test learning`（后者用真正的学习模块端到端验证），红了就改 `remote.rs`。
3. 上游新加了学习表：`remote.rs` 与 `cloud/crates/qingjian-cloud-client/src/learning/table.rs` 各加一种。
