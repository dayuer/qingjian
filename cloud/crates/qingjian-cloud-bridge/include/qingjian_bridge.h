// 青简 Cloud 的 iOS 桥：qingjian-cloud-bridge 静态库的 C 接口，与 src/lib.rs 一一对应。
// 返回 char * 的函数交出所有权，用 qj_string_free 释放；会话只在主线程上用。

#ifndef QINGJIAN_BRIDGE_H
#define QINGJIAN_BRIDGE_H

#include <stdbool.h>
#include <stdint.h>

typedef struct QjSession QjSession;

// data_dir 里要有 dict.qj（lm.qj 可选）；user_dir 可为 NULL（只在内存里学习）；
// config 是 config.toml，NULL 时用 user_dir 下的；cloud_config 指向 cloud.toml，可为 NULL 或不存在（完全离线）。失败返回 NULL。
QjSession *qj_session_open(const char *data_dir, const char *user_dir, const char *config,
                           const char *cloud_config);
void qj_session_free(QjSession *session);

void qj_push(QjSession *session, uint32_t c);
void qj_backspace(QjSession *session);
void qj_clear(QjSession *session);
bool qj_composing(QjSession *session);

char *qj_preedit(QjSession *session);
uint32_t qj_candidate_count(QjSession *session);
char *qj_candidate_text(QjSession *session, uint32_t index);
bool qj_candidate_is_cloud(QjSession *session, uint32_t index);
// 整个候选栏一次取回：每格「'0'/'1'（本地 / 云端）+ 文字」，格间用 0x1E 隔开。
char *qj_candidates(QjSession *session);

char *qj_commit(QjSession *session, uint32_t index);
char *qj_take_raw(QjSession *session);
char *qj_punctuate(QjSession *session, uint32_t c);
void qj_note_passthrough(QjSession *session, uint32_t c);

void qj_flush(QjSession *session);

// 青简 Cloud：上下文、轮询（大模型结果与学习数据收件箱）、同步。
void qj_set_context(QjSession *session, const char *before, const char *after);
bool qj_poll(QjSession *session);
bool qj_cloud_enabled(QjSession *session);
void qj_sync_now(QjSession *session);

// 润色。status：0 空闲、1 等待中、2 就绪、3 失败。
bool qj_rewrite_available(QjSession *session);
void qj_rewrite_start(QjSession *session, const char *text);
uint32_t qj_rewrite_status(QjSession *session);
char *qj_rewrite_take(QjSession *session);
void qj_rewrite_cancel(QjSession *session);

// 私密输入框（验证码、密码、信用卡号）：不学习、不记日志、不发云端，剪贴板与润色也停。
void qj_set_private(QjSession *session, bool private_field);

// 跨设备剪贴板：弹出时 refresh，轮询时取提示；插入或关掉后 handled；用户点按钮才 push 本机剪贴板。
bool qj_clipboard_enabled(QjSession *session);
void qj_clip_refresh(QjSession *session);
char *qj_clip_offer_text(QjSession *session);
char *qj_clip_offer_device(QjSession *session);
void qj_clip_handled(QjSession *session);
void qj_clip_push(QjSession *session, const char *text);

// 主 App 设置页。读返回 JSON（失败为 NULL）；写成功返回 NULL，失败返回原因。
// config.toml 与 Mac 同格式并经青简 Cloud 同步，键盘每次轮询按修改时间重读。
char *qj_settings_read(const char *config_path, const char *dicts_dir);
char *qj_settings_write(const char *config_path, const char *json);
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
// feature：clipboard / sync / input_log / llm；成功后同时写回 cloud.toml 的开关。改 sync 时还会清本机学习数据与配置的同步进度。
char *qj_account_set_consent(const char *path, const char *feature, bool enabled);
char *qj_account_revoke_session(const char *path, int64_t session_id);
// 退出登录：本机总会退出（清令牌与开关），服务器上没注销掉只记日志。
char *qj_account_sign_out(const char *path);
// 删账号：服务器删成功才清本机令牌。
char *qj_account_delete(const char *path);

void qj_string_free(char *text);

#endif
