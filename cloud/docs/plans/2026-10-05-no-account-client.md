# 子项目 1b 默认不要账号：客户端实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.
> 审计会话给出的**任务大纲**。执行前用 writing-plans 展开成逐步代码与命令，先发审计会话「素笺输入法」审过再动手。

**Goal:** 开通云服务不登录；设备与找回页；匹配码加入；Mac 同样支持。

**Spec:** synon-ime `docs/superpowers/specs/2026-10-05-no-account-design.md`。**顺序：** 改名 → 本计划 → 2A Task 6 → 2B → 2C（2B 的同意页挂在本计划的开通流程上）。

## Task 1：proto 与 client（先做并推送，服务端依赖）
proto 加 `CreateSpace`、`PairCode`、`PairJoin`、`PairJoinGrant`、`PairRequest`、`PairDecision`、`PairPoll`、事件 `pair_request` / `device_joined`、错误码 `not_bound` / `identity_taken` / `bad_code` / `device_limit` 与路径常量；client 加对应方法；匹配码规范化函数放 proto（两端共用）。测试：JSON 往返；规范化表驱动。

## Task 2：桥
C ABI：`qj_space_create`、`qj_pair_code`、`qj_pair_join` / `qj_pair_poll`、`qj_pair_requests` / `qj_pair_decide`、绑定三种、找回三种、`qj_identity_remove`。令牌仍不出桥，成功写 `cloud.toml`。测试：假服务端走通建空间、加入、允许、找回。

## Task 3：iOS 开通与新设备
2e「了解云服务」→ 出境同意 → 建空间 → 2f；新设备「输入匹配码 / 用找回方式」；去掉原「账号」页的登录入口。截图对照（审计会话转给 UI 审计员）。

## Task 4：iOS 设备与找回页、允许弹条
「我」→ 设备与找回：设备列表、加一台设备（码 + 倒计时）、绑定 / 解绑、没绑的灰字提示；收到 `pair_request` 时 App 内弹条。微信用 OpenSDK（需用户先在开放平台申请移动应用，未申请时隐藏微信项）。

## Task 5：Mac
菜单「素笺云 ›」：加入…（匹配码 / 网页找回）、出匹配码…、允许通知；建空间。真机验收按仓库约定。

## Task 6：文档
`cloud/docs/design.md` 改账号一节；fork-patch 记上游文件改动。
