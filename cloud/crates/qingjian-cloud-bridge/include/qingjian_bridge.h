// 青简 Cloud 的 iOS 桥：qingjian-cloud-bridge 静态库的 C 接口，与 src/lib.rs 一一对应。
// 返回 char * 的函数交出所有权，用 qj_string_free 释放；会话只在主线程上用。

#ifndef QINGJIAN_BRIDGE_H
#define QINGJIAN_BRIDGE_H

#include <stdbool.h>
#include <stdint.h>

typedef struct QjSession QjSession;

// data_dir 里要有 dict.qj（lm.qj 可选）；user_dir 可为 NULL（只在内存里学习）；
// cloud_config 指向 cloud.toml，可为 NULL 或不存在（完全离线）。失败返回 NULL。
QjSession *qj_session_open(const char *data_dir, const char *user_dir, const char *cloud_config);
void qj_session_free(QjSession *session);

void qj_push(QjSession *session, uint32_t c);
void qj_backspace(QjSession *session);
void qj_clear(QjSession *session);
bool qj_composing(QjSession *session);

char *qj_preedit(QjSession *session);
uint32_t qj_candidate_count(QjSession *session);
char *qj_candidate_text(QjSession *session, uint32_t index);
bool qj_candidate_is_cloud(QjSession *session, uint32_t index);

char *qj_commit(QjSession *session, uint32_t index);
char *qj_take_raw(QjSession *session);
char *qj_punctuate(QjSession *session, uint32_t c);

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

void qj_string_free(char *text);

#endif
