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

// 本地神经整句模型（含章·通变）。加载在后台线程（预热几百毫秒），完成前查询照常；
// 接上后停键的整句重打分生效。state：0 未加载、1 加载中、2 在用、3 上次失败。
bool qj_load_model(QjSession *session, const char *path, bool p2c);
uint8_t qj_model_state(QjSession *session);
void qj_unload_model(QjSession *session);
// 内存吃紧先卸英文表（约 13MB），下次像英文的输入自动再加载；还紧再卸模型。
void qj_unload_english(QjSession *session);
// 模型自报的内存占用（MB）；没加载返回 0。调试面板显示用。
double qj_model_memory_mb(QjSession *session);
// 本进程剩余可用内存（MB，os_proc_available_memory）；拿不到是 -1。键盘侧内存自保用它。
double qj_available_memory_mb(void);
// 素材会被整理：path（cloud.toml）里有服务器地址和登录令牌，并且同意了「记忆」（memory = true），不联网；
// 文件不在、没登录、没同意或参数无效为 false。App 的「待整理」引导与键盘记一笔的 toast 按它判断。
bool qj_memory_cloud_ready(const char *path);
// 素材与输入日志的后台上传：App 前台踢一脚（节流在线程里）；上传器随键盘会话活着。
void qj_upload_kick(const char *user_dir);
// 素材交给谁整理（同意页用）：{"name","zero_retention"}；连不上 NULL。
char *qj_memory_processor(const char *cloud_path);
// 清空云端输入记录并删本机日志与进度；成功 NULL。
char *qj_input_log_clear(const char *user_dir, const char *cloud_path);
void qj_sync_now(QjSession *session);

// 润色。status：0 空闲、1 等待中、2 就绪、3 失败（网络失败、服务器没开大模型）、4 模型给的不合用（已丢掉，换一个技能再试）。
// 技能包随 App 包走（Data/skills），会话打开时读一次。包里的技能一个都没有时改整个用不了：available 为 false、skills 为 NULL。
bool qj_rewrite_available(QjSession *session);
// 可用的改写技能：JSON 数组 [{"id","name","summary"}]，按 order 排；提示词不下发到壳里（壳只用来显示名字）。
// 一个都没有或会话无效时为 NULL。用了哪个技能由键盘自己算（它知道当前选的人），桥不回传。
char *qj_rewrite_skills(QjSession *session);
// skill_id 为空指针或认不得时用当前生效的那个（选中的人的技能 → 设置里的默认 → 列表第一个）。
void qj_rewrite_start(QjSession *session, const char *text, const char *skill_id);
uint32_t qj_rewrite_status(QjSession *session);
char *qj_rewrite_take(QjSession *session);
void qj_rewrite_cancel(QjSession *session);
// 改写用的全局默认技能（config.toml 的 [rewrite] skill，与主 App 设置页的 rewrite_skill 是同一项，见 qj_settings_read）：
// 读 {"skill":"polish"}；会话无效、或这个会话没有配置文件时为 NULL；文件里存的不是合法技能编号时静默回退成缺省（读不报错）。
// 键盘按「选中的人的技能 → 这一项 → 列表第一个」挑，链里第一环见 qj_memory_contact_skill。
char *qj_rewrite_default(QjSession *session);
// 改全局默认技能（skill_id 为 NULL = 回到缺省 polish）：成功返回 NULL，失败返回 {"code","message"}。
// 只校验编号的形状（小写字母、数字、- 与 _，不超过 32 个），不校验这个技能现在在不在——技能包随版本增删，
// 写进来一个暂时认不得的编号由键盘回退；写回去只动 [rewrite] skill 这一项，配置文件里别的内容与注释原样保留。
char *qj_rewrite_default_set(QjSession *session, const char *skill_id);

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
// JSON 里的 rewrite_skill 是改写用的默认技能（技能包 id，缺省 polish；某个人身上指定了就用他的），
// 落在 config.toml 的 [rewrite] skill——这个分节只有 iOS 用，Mac 读配置时按分节取，多出来的忽略。
char *qj_settings_read(const char *config_path, const char *dicts_dir);
char *qj_settings_write(const char *config_path, const char *json);
// 账号（主 App 用）：path 是 App Group 里的 cloud.toml；都是阻塞的网络请求，在后台线程调。令牌只在 cloud.toml 与桥之间流转。
// status 返回 JSON（参数无效时为 NULL，没登录时不联网；取不到账号时带 error 文案与 error_code，没有错误时两者都省略）。
// 其余成功返回 NULL，失败返回 JSON {"code":"…","message":"…"}：message 是给用户看的中文，code 取值
// auth_failed / locked_today / consent_required / unauthorized / not_configured / rate_limited / forbidden / unreachable /
// invalid_argument / not_signed_in / other；status 的 error_code 取值相同。键盘下次弹出时按新的 cloud.toml 重连。
// 本机有没有拿到过会话：只读 cloud.toml 的令牌，不联网。App 用它决定「我」页那一行显示已开通还是没开通。
bool qj_cloud_signed_in(const char *path);
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

// 空间与匹配码（素笺 1b「不要账号」，取代登录）：开通云服务时建空间，新设备用匹配码申请加入，旧设备上允许之后才拿到会话。
// path 同上；都是阻塞的网络请求，在后台线程调。令牌与登录一样只在 cloud.toml 与桥之间流转。
// 成功的返回值里没有令牌，也不含失败 JSON 的那个 code 字段：qj_pair_code 给的是 pair_code。
// 建空间成功后这台设备就已登录（令牌写进 cloud.toml），qj_pair_poll 收到 approved 时同样。
// device 是设备名，可为 NULL（缺省 "iPhone"）；cross_border_consented 为假时不联网，直接返回 consent_required。
char *qj_space_create(const char *path, const char *device, bool cross_border_consented);
// 出一张匹配码（要已登录）：成功 {"pair_code":"K7P2-9QXM","expires_at":…}（毫秒）。
char *qj_pair_code(const char *path);
// 新设备输码申请加入：成功 {"request_id":…,"secret":…,"expires_at":…}，secret 轮询时回传。
// 码不对、过期返回 code 为 bad_code 的失败；空间满 5 台返回 device_limit（码不被消费，还能给别人用）。
char *qj_pair_join(const char *path, const char *code, const char *device);
// 轮询这次申请：成功 {"state":"pending"|"denied"|"approved"}；approved 时会话已写进 cloud.toml。
char *qj_pair_poll(const char *path, const char *request_id, const char *secret);
// 等这台设备处理的加入申请（要已登录）：成功是 JSON 数组 [{"id","name","platform","at"}]。
char *qj_pair_requests(const char *path);
// 允许或拒绝一条加入申请（要已登录）：allow 为真之后新设备才取得到令牌。
char *qj_pair_decide(const char *path, const char *request_id, bool allow);

// 本地记忆（素笺 2A）：对象、打字提示、对象卡、「记一笔」（存成待整理素材）。会话没有学习数据目录（user_dir 为 NULL）时都是空操作 / 返回 NULL。
// App 与键盘的读-改-写都在 memory/.lock 的文件锁里做。名单是一张平铺的人，人数不限，全局可以置顶最多 4 个人。
// contact_id 是 32 位十六进制；NULL 表示保持现在选的人不变（幂等），空字符串 "" 表示明确不指定；
// 磁盘名单上没有的对象当不指定；切到了某人时记下时间（used，列人时按沟通情况排用）。
// 切换时在锁里重读 memory/state.json、只改当前对象再写回；开机后还没解锁过、读不了就不切。候选按新的分区学习重排。
// 键盘只等 200 毫秒的锁：拿不到时内存里照切，写盘进待办（只留最新一次），下次按键、qj_poll、qj_flush 时补写。
void qj_scope_set(QjSession *session, const char *contact_id);
// {"contact_id":"…"|null,"used":{"<id>":1791043200}}（used：各人上次被选中的 Unix 秒）
char *qj_scope_get(QjSession *session);
// 宿主换了输入框时调：清掉最近上屏的字与正在显示的匹配提示（qj_flush 也会清）。
void qj_reset_context(QjSession *session);
// 当前提示 {"card_id","text","reason":"match"|"today","more":bool}（more：除了这张还有别的卡）；
// 没有、私密输入、没选对象或这个人的开关关着时为 NULL。
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
// （最多 32 条，落盘在 memory/pending-keyboard.jsonl），下次按键、qj_poll、qj_flush 或下一次记一笔时按顺序补写，主线程不会卡住。
// 排满了挤掉最旧的、补写时被拒绝的（素材满了、对象被忘掉）都不悄悄丢，条数与原因记下来，用 qj_memory_dropped 取。
char *qj_memory_note(QjSession *session, const char *contact_id, const char *text, const char *source);
// 待办补写时被拒绝、没记上的条数（按切好的素材条数算，只有条数与原因，没有原文），取走即清零（落盘在 memory/dropped-keyboard.json，
// 键盘被杀也不丢）；键盘出现时调，提示一次：{"material_limit":n,"contact_gone":n,"queue_full":n}（material_limit：这个人的待整理满了；
// contact_gone：这个人已经被忘掉；queue_full：排队等补写的超过 32 条，最旧的被挤掉）。都是 0、会话没有记忆目录或参数无效时为 NULL。
char *qj_memory_dropped(QjSession *session);
// 键盘里新建一个对象（名字与称呼，称呼由 App 里改）：成功返回 {"id":"…"}，失败返回 {"code","message"}
// （lock_timeout：App 正占着锁，请再点一次；invalid：名字为空）。人数不限。
// pronoun 取 ta / ta_m / ta_f / name，NULL 或认不得按 ta。
char *qj_memory_add_contact(QjSession *session, const char *name, const char *pronoun);
// App 用，user_dir 是 App Group 里的 Qingjian 目录（记忆在它下面的 memory/）。read 返回
// {"contacts":[…],"cards":{id:[…]},"revs":{id:n},"state":{…},"broken":[id…]}（revs 是各对象卡片的修订号；
// broken 是卡片文件损坏、已备份的对象；参数无效或有文件读不了时为 NULL）。
// write 整份写回：成功返回 NULL，失败返回 {"code","message"}，code 取 pin_limit / invalid / conflict / lock_timeout / io（lock_timeout：App 等了 2 秒还拿不到锁，稍后再试）。
// 某个对象磁盘上的修订号比 revs 新（这期间别处改过卡片）就整份不写、返回 conflict，App 重读合并后再写；
// 只重写有变化的对象；state 不采纳；名单上没了的对象连目录一起删（卡片、素材、分区学习都在里面）。
// 置顶全局超过 4 个返回 pin_limit。
// 卡片不合格（关键词长短、日期格式等）也是 invalid，message 写明是谁的哪张卡：「小美的卡「不吃香菜」：每个关键词要 2 到 8 个字」。
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

// App 用：**还没归到人的**素材（首页「+ 记一条」先记下的），与 qj_memory_materials 同形、同样「新的在上」；
// 存在 memory/unassigned.jsonl（不绑对象；不能放进 <伪对象 id>/，写快照时会把名单上没有的人的目录连内容删掉）。
// 参数无效或读不了时为 NULL。
char *qj_memory_unassigned_materials(const char *user_dir);
// App 用：首页「+ 记一条」——把一句话存成不绑对象的待整理素材（切段与上限同键盘的「记一笔」）。
// 成功返回 NULL（与 qj_memory_note 一样；要看内容调 qj_memory_unassigned_materials）；
// 失败返回 {"code","message"}（material_limit 另带 remaining/needed、invalid / lock_timeout / io）。
// source 取 "clipboard" / "typed"，NULL 或不认得按 typed。
char *qj_memory_unassigned_note(const char *user_dir, const char *text, const char *source);
// App 用：「补上」——把无主素材里的一条归到某个人名下。成功返回 NULL，失败返回 {"code","message"}
// （invalid：人不在名单上或无主素材里没有这条；lock_timeout / io）。
char *qj_memory_assign_material(const char *user_dir, const char *client_id, const char *contact_id);

// App 用：这个人改写用哪个技能。技能 id 定在技能包里，指定跟着人走——导出记忆、备份、换机都带着它；
// 没指定就用设置里的默认（qj_settings_read 的 rewrite_skill）。
// 读：{"skill":"tactful"}，没指定时 {"skill":null}；参数无效、名单上没有这个人或读不了时为 NULL。
char *qj_memory_contact_skill(const char *user_dir, const char *contact_id);
// 指定 / 清掉（skill_id 为 NULL = 清掉，回到设置里的默认）：成功返回 NULL，失败返回 {"code","message"}
// （invalid：技能编号不对或名单上没有这个人；lock_timeout / io）。
char *qj_memory_contact_skill_set(const char *user_dir, const char *contact_id,
                                  const char *skill_id);
// App 用：读 skills_dir 下随包的改写技能（App 包里那份，由 cloud/ios/project.yml 把 assets/skills 打成 skills/）：
// JSON 数组 [{"id","name","summary"}]，按 order 排（与 qj_rewrite_skills 同形，只是读的是另一个目录）；
// 提示词不下发到壳里；目录里一个都没有或参数无效时为 NULL。App 只用来列出技能名（改写本身在键盘里）。
char *qj_skills(const char *skills_dir);

// 键盘工具栏「记录中」（05 的 2a / 2b）。0 不显示（没登录、没开上传输入日志）、1 记录中、
// 2 暂停（1 小时后自动恢复）、3 一直暂停。问它时顺带处理到期。
// 暂停只停记输入日志，学习、提示、记一笔照常；已经记下的照常上传。
uint8_t qj_recording_state(QjSession *session);
// seconds > 0 暂停这么多秒；<= 0 一直暂停（到 qj_recording_resume）。
void qj_recording_pause(QjSession *session, int64_t seconds);
// 恢复记录（点「已暂停」）。
void qj_recording_resume(QjSession *session);

void qj_string_free(char *text);

#endif
