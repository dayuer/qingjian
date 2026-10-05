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
// 开了素笺云：path（cloud.toml）里有服务器地址和登录令牌，不联网；文件不在、没登录或参数无效为 false。
// App 的「待整理」引导与键盘记一笔的 toast 按它判断（与键盘建云端客户端时的判断一样）。
bool qj_cloud_configured(const char *path);
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
// status 返回 JSON（参数无效时为 NULL，没登录时不联网；取不到账号时带 error 文案与 error_code，没有错误时两者都省略）。
// 其余成功返回 NULL，失败返回 JSON {"code":"…","message":"…"}：message 是给用户看的中文，code 取值
// auth_failed / locked_today / consent_required / unauthorized / not_configured / rate_limited / forbidden / unreachable /
// invalid_argument / not_signed_in / other；status 的 error_code 取值相同。键盘下次弹出时按新的 cloud.toml 重连。
char *qj_account_status(const char *path);
// nonce 是原始值（交给 Apple 的是它的 SHA-256 十六进制）；device 是设备名，可为 NULL。
// 三个登录函数的 cross_border_consented 是用户是否勾选了「同意把数据发到境外服务器」：false 时不联网，
// 直接返回 code 为 consent_required 的失败；true 时桥在请求里带当前的同意文本版本。
char *qj_account_sign_in_apple(const char *path, const char *identity_token,
                               const char *authorization_code, const char *nonce,
                               const char *device, bool cross_border_consented);
char *qj_account_email_start(const char *path, const char *email, bool cross_border_consented);
char *qj_account_email_verify(const char *path, const char *email, const char *code,
                              const char *device, bool cross_border_consented);
// feature：clipboard / sync / input_log / llm；成功后同时写回 cloud.toml 的开关。改 sync 时还会清本机学习数据与配置的同步进度。
char *qj_account_set_consent(const char *path, const char *feature, bool enabled);
char *qj_account_revoke_session(const char *path, int64_t session_id);
// 退出登录：本机总会退出（清令牌与开关），服务器上没注销掉只记日志。
char *qj_account_sign_out(const char *path);
// 删账号：服务器删成功才清本机令牌。
char *qj_account_delete(const char *path);

// 本地记忆（素笺 2A）：场景、对象、打字提示、对象卡、「记一笔」（存成待整理素材）。会话没有学习数据目录（user_dir 为 NULL）时都是空操作 / 返回 NULL。
// App 与键盘的读-改-写都在 memory/.lock 的文件锁里做。
// 每个场景各有一组人（各自最多 8 个，互不相通），对象建好后不能换场景。
// scene 取 daily / dating / work；contact_id 是 32 位十六进制；NULL 表示回到这个场景上次选的人，空字符串 "" 表示明确不指定；
// 不是这个场景的人、磁盘名单上没有的对象都当不指定。
// 切换时在锁里重读 memory/state.json、只改场景与对象（与各场景上次选的人）再写回；开机后还没解锁过、读不了就不切。候选按新的分区学习重排。
// 键盘只等 200 毫秒的锁：拿不到时内存里照切，写盘进待办（只留最新一次），下次按键、qj_poll、qj_flush 时补写。
void qj_scope_set(QjSession *session, const char *scene, const char *contact_id);
// {"scene":"dating","contact_id":"…"|null,"last":{"dating":"…","daily":"…"},"used":{"<id>":1791043200}}（last：各场景上次选的人；used：各人上次被选中的 Unix 秒）
char *qj_scope_get(QjSession *session);
// 宿主换了输入框时调：清掉最近上屏的字与正在显示的匹配提示（qj_flush 也会清）。
void qj_reset_context(QjSession *session);
// 当前提示 {"card_id","text","reason":"match"|"today","more":bool}（more：除了这张还有别的卡）；
// 没有、私密输入、工作场景、没选对象或这个人的开关关着时为 NULL（恋爱与日常出提示，工作不出）。
char *qj_memory_hint(QjSession *session);
// 「知道了」：today 为 true 时当天不再出这张卡（记进 memory/dismissed.json），false 时 10 分钟内不再出。
void qj_memory_dismiss(QjSession *session, const char *card_id, bool today);
// 键盘内对象卡面板：今日相关最多 3 张卡的 JSON 数组。
char *qj_memory_cards(QjSession *session, const char *contact_id);
// 「记一笔」：原话原样存成这个对象的一条待整理素材（memory/<对象 id>/materials.jsonl），不再写卡；超过 2000 字节的先按空行、
// 单段再按字节（不切断字符）切成几条，一个字不丢。source 取 clipboard / typed，NULL 或认不得按 typed。
// 成功返回 NULL，失败返回 {"code","message"}（invalid：没有这个人或没有文字；io：素材读不了，例如开机后还没解锁过，此时什么都不写）。
// material_limit：这次切出的条数加上没整理的超过 200 条，整次一条都不写，另带 remaining（还剩几个空位）与 needed（这次要几条）：
// {"code":"material_limit","message":"这次有 2 条，这个人只剩 1 个空位，先去 App 里整理","remaining":1,"needed":2}，
// remaining 为 0 时 message 是「这个人还有 200 条没整理，先去 App 里看看」。
// 键盘只等 200 毫秒的锁：另一个进程占着锁（lock_timeout）时也返回 NULL，表示已接受、稍后写入：这条记在待办里
// （最多 32 条，满了丢最旧的，落盘在 memory/pending-keyboard.jsonl），下次按键、qj_poll、qj_flush 或下一次记一笔时按顺序补写，主线程不会卡住。
// 补写时被拒绝的（素材满了、对象被忘掉）不悄悄丢，条数与原因记下来，用 qj_memory_dropped 取。
char *qj_memory_note(QjSession *session, const char *contact_id, const char *text, const char *source);
// 待办补写时被拒绝、没记上的条数（按切好的素材条数算，只有条数与原因，没有原文），取走即清零（落盘在 memory/dropped-keyboard.json，
// 键盘被杀也不丢）；键盘出现时调，提示一次：{"material_limit":n,"contact_gone":n}（material_limit：这个人的待整理满了；
// contact_gone：这个人已经被忘掉）。都是 0、会话没有记忆目录或参数无效时为 NULL。
char *qj_memory_dropped(QjSession *session);
// 键盘里在 scene 新建一个对象：成功返回 {"id":"…"}，失败返回 {"code","message"}（contact_limit：这个场景已满 8 个，
// message 带场景名，如「日常最多 8 个人」；lock_timeout：App 正占着锁，请再点一次；invalid：名字为空）。
// pronoun 取 ta / ta_m / ta_f / name，NULL 或认不得按 ta；scene 为 NULL 或认不得时用会话当前的场景。
char *qj_memory_add_contact(QjSession *session, const char *name, const char *pronoun, const char *scene);
// App 用，user_dir 是 App Group 里的 Qingjian 目录（记忆在它下面的 memory/）。read 返回
// {"contacts":[…],"cards":{id:[…]},"revs":{id:n},"state":{…},"broken":[id…]}（revs 是各对象卡片的修订号；broken 是卡片文件损坏、
// 已备份的对象；参数无效或有文件读不了时为 NULL）。
// write 整份写回：成功返回 NULL，失败返回 {"code","message"}，code 取 contact_limit / invalid / conflict / lock_timeout / io（lock_timeout：App 等了 2 秒还拿不到锁，稍后再试）。
// 某个对象磁盘上的修订号比 revs 新（这期间别处改过卡片）就整份不写、返回 conflict，App 重读合并后再写；
// 只重写有变化的对象；state 不采纳；名单上没了的对象连目录一起删（卡片、素材、分区学习都在里面）；已有的对象换了场景返回 invalid（换场景需要忘掉后重新加）。
char *qj_memory_read(const char *user_dir);
char *qj_memory_write(const char *user_dir, const char *json);
// App 用：一个对象没整理的素材，按时间倒序（同一秒的按写入倒序），整理过 30 天的顺手删掉：
// {"unprocessed_count":n,"materials":[{"client_id":"…","kind":"note","text":"原话","at":1791043200,
//   "source":"clipboard"|"typed","uploaded":false,"processed":false},…]}
// unprocessed_count 等于 materials 的条数（App 的「待整理 · n 条」与 180 条提示按它）；没有素材时 materials 为空数组；
// 参数无效或读不了（开机后还没解锁过、等了 2 秒没拿到锁）时为 NULL。
char *qj_memory_materials(const char *user_dir, const char *contact_id);
// App 用：删一条素材，没有这条也算成功。成功返回 NULL，失败返回 {"code","message"}（invalid / lock_timeout / io）。
char *qj_memory_material_delete(const char *user_dir, const char *contact_id, const char *client_id);

void qj_string_free(char *text);

#endif
