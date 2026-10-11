# A1 包加载崩溃隔离 / A2 包的增量同步 —— WeType 2.2.3 取证与青简复刻规格

材料：`/tmp/wetype-rea/{WeType.arm64, full_otool.asm, nmn.txt, str.txt}`。地址都是 arm64 片的 vmaddr。
Swift 函数名用 `xcrun swift-demangle --simplified` 还原；ObjC 属性表（protobuf 字段）用自写脚本按 `__OBJC_$_PROP_LIST_*` 解析。
工作文件（函数体切片、脚本）在 `/tmp/wetype-rea-a1a2/`。本机 `~/Library/Application Support/WeType/DictUpdate/` 只看了目录名（没读文件内容）。

先说两条和任务原文（docs/notes/wetype-feature-prompts.md）不一致的地方，都已读到指令：

1. WeType 的「崩溃哈希」不是按单个文件、也不是持久黑名单：它记的是**整批词库集合的哈希**，命中后**整批回退到随包词库**，5 秒后标记就清空，下一次检查更新还会把同一版再下回来。
2. WeType **不保留上一版下载的词库**：清理时删掉所有低于当前在用版本的目录；回退目标始终是随包（`imeData.bundle`）那一份。补丁失败也**不会当场改下整包**，而是整批下载中止，等下一轮检查。

青简按任务原文的要求（按包、持久拉黑到新版本、保留上一版）做，比 WeType 更细；下文「青简落点」里写明了哪些是 WeType 的做法、哪些是青简自己加的。

---

## A1 包加载崩溃隔离

### 1. WeType 怎么做的

**入口**：`DictsManager.doReloadDicts()` @0x100296348（`WeType/Module/DictUpdate/DictsManager.swift`）。
调用方 [确]：
- `static Engine.setUpEngine()` 内 0x10023bb5c（启动建引擎时）
- `closure #1 in closure #1 in DictsManager.init()` 内 0x100298868（下载全部完成的通知回调里）
- `DebugViewController` 内 0x100170b38（调试菜单）

**数据**
- 标记：`DictsConfig._maybeCrashDictsHashes: Set<String>`（字段偏移 `direct field offset` @0x102e24a80，实例内 +0xe8）[确]。
- 持久化：MMKV 实例 `commonCryptedMMKV`（全局 @0x10335b8f0，懒初始化 0x1000e8c68，mmapID 以 `wetype.common…` 开头、带 `StringMixUtil.commonKey` 加密钥）[确]，
  键 `"MaybeCrashDictsHash"`，值是 `NSMutableSet(set)`，经 `-[MMKV setObject:forKey:]` 写 [确]。启动时 `DictsConfig.loadFromMMKV()` @0x10042a7e4 用 `getObjectOfClass:forKey:` 读回，
  日志 `[Dict][loadMMKV] did load maybeCrashDictsHashes: ` [确]。
- 哈希是什么：`DictsUtil.dictsHash(_:[DictInfo])` @0x10028cd64 对**整批可用词库**算出的一个字符串；函数里只读每个 `DictInfo` 的 `md5` 字段，
  最后对拼出来的 `Data` 调 `-[NSData md5]` [确：selector]。所以它是「这批文件内容 md5 的汇总再 md5」[推]（具体怎么折叠 md5 串没读完，见未知）。
  不是 cellHash、也不是路径哈希。cellHash / normalHash 是同一个函数分别作用于子集，用来判断是否要重载。
- 迁移开关：`commonMMKV`（@0x10335b8e8）里的 Int32 键 `"DIctsLogicVer"`（原文就是大写 I 的拼写）[确：0x1002966a0 处立即数解码]；不等于 1 时也走强制回退，`_cleanDicts()` 里把它写成 1。

**控制流（doReloadDicts，地址在注释里）**

```swift
func doReloadDicts() {
    log("[Dict][Main] reload dicts")
    var dicts = getAvailableDicts()                       // 0x1002964ec：下载的 + 随包的合并后、过滤掉引擎版本不匹配的
    let allHash = DictsUtil.dictsHash(dicts)              // 0x1002964f4
    let last = lock { _lastLoadedAllDictsHash }           // os_unfair_lock，DictsManager+0x60
    if !last.isEmpty && last == allHash {                 // 0x10029656c..0x100296608
        logInfo("…")                                       // 行号 0x51；这条路径直接 return，不登记标记、不跑 defer
        return
    }
    let logicOK = commonMMKV.getInt32("DIctsLogicVer") == 1          // 0x1002966f0
    let crashed = logicOK && config.maybeCrashDictsHashes.contains(allHash) // 0x100296730 Set.contains
    if !logicOK || crashed {
        log("[Dict][Main][reloadDicts] force load local dicts, " +
            (logicOK ? "because last crashed" : "because force clean"))     // 0x10029687c cinc/csel 选串
        clearDownloadedDicts(resetUpdateTime: false)      // 0x1002968f0，w0 = 0
        dicts = getLocalAvailableDicts()                  // 0x1002968f4：只剩随包词库
    }
    // —— 登记「正在加载」——
    config.maybeCrashDictsHashes.insert(allHash)          // 0x100296948；注意插的是回退前算的 allHash
    logDebug("[Dict][Config] set _maybeCrashDictsHashes = …")  // DictsConfig.swift:86
    commonCryptedMMKV.setObject(NSMutableSet(config.maybeCrashDictsHashes), forKey: "MaybeCrashDictsHash") // 0x100296a90
    defer { scheduleClear() }                              // $defer @0x1002975a8，在函数尾 0x100297538 调
    // —— 真正加载 ——
    var normal: [DictInfo] = [], cell: [DictInfo] = [], loaded: [[String: Any]] = []
    for d in dicts {                                       // 0x100296ba4 循环，步长 0x68 = sizeof(DictInfo)
        guard FileManager.default.fileExists(atPath: d.path) else {
            log("[Dict][Main][reloadDicts] path not exists: " + d.fullDesc); continue   // 单个缺文件只跳过
        }
        if d.dictType == 1 { cell.append(d) } else { normal.append(d) }    // DictInfo+0x4c，cmp w20,#1 @0x100296c3c
        loaded.append(["name": d.name, "version": d.version])               // 0x100296e60 / 0x100296e80 立即数
    }
    if DictsUtil.dictsHash(normal) != loadedNormalDictsHash {
        log("[Dict][Main][reloadDicts] normalHash different")
        Engine.addCommonDicts(normal)                      // 0x10029717c
    }
    if DictsUtil.dictsHash(cell) != loadedCellDictsHash || dictChanged {
        log("[Dict][Main][reloadDicts] cellHash different or dictChanged")
        Engine.addCellDicts(cell)                          // 0x100297344
    }
    loadDictsInfo = JSONSerialization.data(loaded)        // 0x100297434，日志 "[Dict][Main][reloadDicts] loaded dicts = "
}

// $defer #1 @0x1002975a8
func scheduleClear() {
    DispatchQueue.main.asyncAfter(deadline: .now() + 5.0) {   // fmov d0, #5.0 @0x1002976c0
        // closure @0x100298c94
        config.maybeCrashDictsHashes = []                       // 写空集合 __swiftEmptySetSingleton 到 +0xe8
        commonCryptedMMKV.setObject(NSMutableSet([]), forKey: "MaybeCrashDictsHash")
        logInfo("[Dict][Main] did reload all dicts …")          // DictsManager.swift:81 处
    }
}
```

要点：
- **写入时机**：真正调引擎加载之前、只在「这批和上次不同」时写。
- **清除时机**：本函数返回（引擎加载是同步调用，返回即加载完）后在主队列延迟 **5 秒**清空整个集合 [确]。没有「处理过 N 次按键」之类的条件，纯时间。
  Mac 上输入法进程常驻，5 秒内没崩就算稳定。
- **命中后**：整批回退到随包词库；`clearDownloadedDicts(resetUpdateTime:false)` @0x1002934e8 只把 MMKV 里
  `CommonDictsUpdateInfo` / `NewHotDictsUpdateInfo` 的 `dicts` 清成 `[]` 并重新编码写回 [确]，**不删文件**（文件留给下次 `_cleanDicts` 按「不在配置里」删）；
  `resetUpdateTime` 为 false，所以 `nextUpdateTime` 不清零，要等下一次正常检查周期（≥1 小时，见 A2）才会重新拉 [确：0x100293704 fcsel]。
- **何时解除**：回退那一次加载同样会在 5 秒后清空集合，所以「黑名单」只活一个启动周期；下次检查更新若服务端仍给同一版，会再下、再试 [推]。
- **上报**：`doReloadDicts` 里没有任何 `BIZ.report*` 调用，只有 xlog 日志 [确：函数内调用清单]。崩溃本身另由 Sentry（`SentrySDKWrapper`）收，跟这条逻辑无关。
- 相关的「重载去重」：`_lastLoadedAllDictsHash` 相同就整段跳过，所以标记只在内容真的变了才登记，不会每次启动都登记。

### 2. 证据表

| 结论 | 置信度 | 地址 / 字符串 |
|---|---|---|
| 标记字段 `DictsConfig._maybeCrashDictsHashes` | 确 | field offset 符号 @0x102e24a80；str.txt `_maybeCrashDictsHashes` |
| MMKV 键 `MaybeCrashDictsHash`，值 NSMutableSet | 确 | 0x100296a60 / 0x100298dc8 / 0x10042aae8 三处字面量引用；`initWithSet:` + `setObject:forKey:` |
| 存在 `commonCryptedMMKV` | 确 | 全局 @0x10335b8f0，nmn 符号 `commonCryptedMMKV` |
| 加载前登记 allHash | 确 | `Set._Variant.insert` @0x100296948，参数 `[x19+0x10]` = 0x1002964f4 处 `dictsHash` 结果 |
| 加载后 5 秒清空（主队列 asyncAfter） | 确 | `$defer` @0x1002975a8：`DispatchTime.now()` + `fmov d0,#5.0` + `asyncAfter`；闭包 @0x100298c94 写空集合 |
| 命中 → 「because last crashed」→ 整批改用随包词库 | 确 | 0x100296854/0x100296874 字串；`clearDownloadedDicts` @0x1002968f0；`getLocalAvailableDicts` @0x1002968f4 |
| `DIctsLogicVer != 1` → 「because force clean」 | 确 | 0x1002966a0 立即数 `DIctsLog`+`icVer`；`cmp w22,#1` @0x1002966f0 |
| 回退不重置更新时间 | 确 | 调用时 `mov w0,#0` @0x1002968e8；clearDownloadedDicts 内 `fcsel` @0x100293704 |
| 哈希只依赖各词库 md5 | 推 | dictsHash 内唯一读取的属性是 `md5` @0x10028d0f0 |
| 不上报 | 确（限 doReloadDicts 本身） | 函数调用清单里无 `BIZ.*` |
| 单个文件缺失只跳过、不触发回退 | 确 | `[Dict][Main][reloadDicts] path not exists: ` @0x100296b70 分支 |
| dictType==1 归为细胞词库 | 确 | DictInfo+0x4c，与 CodingKeys 顺序推得的布局一致（path 在 +0x58，patchDict 取的正是 +0x58） |

### 3. 青简落点

**现状**（先 grep 过，没有编造）：
- 人设包 / 用户包还没实现：`cloud/docs/specs/relationship-memory.md` 第 9 行写「人设包 Persona…仍未实现」，第 99 行「包带版本号，同步走增量…」，第 215–216 行定了 `GET /packs`、`GET /packs/{id}?from=<版本>`。
  `cloud/crates/*/src` 里 grep `Pack|packs|persona` 没有实现代码。
- 现有的加载路径：iOS `cloud/ios/Keyboard/Sources/KeyboardViewController.swift` 的 `openEngine(fullAccess:)` → `Engine.init`（`cloud/ios/Keyboard/Sources/Engine.swift`）
  → C ABI `qj_session_open`（`cloud/crates/qingjian-cloud-bridge/src/lib.rs:51`，已有 `catch_unwind`，但接不住 jetsam / abort / 越界读 mmap 的 SIGBUS）
  → `Session::open`（`cloud/crates/qingjian-cloud-bridge/src/session/mod.rs:94`：`dict.qj`、`lm.qj`、懒加载 `english.qj`、`skills/`、`memory/` 经 `LiveMemory::open`），
  领域词库经 `session/config.rs:70` 调 `qingjian_platform::extra_dictionaries::load`（`crates/qingjian-platform/src/extra_dictionaries.rs`，坏文件已经「只记日志跳过」，但只防 `Err`，防不住崩溃）。
- App Group 目录：`cloud/ios/Shared/SharedStore.swift`（`group.sujian.synon.ai`，`Library/Application Support/Qingjian`）。Mac 的数据目录见 `cloud/crates/qingjian-cloud-mac/src/paths.rs`。
- 平台无关的云端客户端 crate 是 `cloud/crates/qingjian-cloud-client`（bridge 与 mac 都依赖它，lib.rs 写明「不依赖任何平台 API」），崩溃隔离与包仓库放这里最合适。

**放哪**：不进 `qingjian-core`（Core 不知道包从哪来，spec 第 105 行），也不进上游 crates（分叉隔离规则）。新增模块 `cloud/crates/qingjian-cloud-client/src/packs/`：

| 文件 | 内容 |
|---|---|
| `packs/mod.rs` | `mod` 声明与 re-export、常数 |
| `packs/load_guard.rs` | `LoadGuard`：登记 / 清除 / 检查残留 |
| `packs/guard_record.rs` | `GuardRecord`（磁盘上的一条记录） |
| `packs/blocklist.rs` | `Blocklist`：拉黑记录读写、按新版本解除 |
| `packs/pack_store.rs` | `PackStore`：A2 的版本目录、当前指针、上一版；A1 回退要用 |
| `packs/current.rs` | `Current`（当前指针文件的结构） |
| `packs/tests.rs` | 单测（超 200 行就搬到这里） |

**类型与签名（Rust）**

```rust
// packs/mod.rs
/// 加载后要处理过这么多次按键才算稳定；与 STABLE_AFTER 先到者为准。WeType 只用时间（5 秒，Mac 常驻进程），青简按 iOS 键盘扩展的情况自己定，先取 20 / 10 秒，真机后重扫。
pub const STABLE_KEYS: u32 = 20;
pub const STABLE_AFTER: Duration = Duration::from_secs(10);

// packs/guard_record.rs
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct GuardRecord {
    pub pack_id: String,

    pub version: u64,

    /// 包文件的 sha256（小写 hex），与 A2 校验用的是同一个值。
    pub sha256: String,

    /// 登记时刻，Unix 秒。
    pub started_at: u64,
}

// packs/load_guard.rs
pub struct LoadGuard { path: PathBuf /* <group>/packs/loading.json */ }
impl LoadGuard {
    pub fn new(packs_dir: &Path) -> Self;
    /// 启动时调一次：读出上次没清掉的记录（= 上次加载这些包时进程死了），并删掉文件。
    pub fn take_leftovers(&self) -> Vec<GuardRecord>;
    /// 加载某个包之前调：追加一条记录，写临时文件再 rename；只防进程崩溃，不 fsync。
    pub fn begin(&self, record: GuardRecord) -> io::Result<()>;
    /// 稳定后调：清掉这个包的记录（全清了就删文件）。
    pub fn settle(&self, pack_id: &str) -> io::Result<()>;
}

// packs/blocklist.rs
#[derive(Serialize, Deserialize, Default)]
pub struct Blocklist { entries: Vec<GuardRecord> }      // 只存 pack_id / version / sha256 / 时间，不存包内容
impl Blocklist {
    pub fn load(path: &Path) -> Self;                    // <group>/packs/blocked.json，坏文件当空
    pub fn save(&self, path: &Path) -> io::Result<()>;
    pub fn block(&mut self, record: GuardRecord);
    pub fn is_blocked(&self, pack_id: &str, sha256: &str) -> bool;
    /// 同步下来同一个包的新版本（sha256 不同）时调：解除这个包的旧拉黑。返回是否有解除。
    pub fn release_on_new_version(&mut self, pack_id: &str, new_sha256: &str) -> bool;
    /// 上报用：拉黑过、还没报的 (pack_id, version)。
    pub fn unreported(&self) -> Vec<(String, u64)>;
    pub fn mark_reported(&mut self, pack_id: &str, version: u64);
}

// packs/pack_store.rs（A2 也用）
pub enum Resolved { Current(PathBuf), RolledBack(PathBuf), Bundled, None }
impl PackStore {
    /// 选本次要加载的文件：当前版本被拉黑就退上一版；上一版也拉黑或没有，人设包退随包底包、用户包不加载。
    pub fn resolve(&self, pack_id: &str, blocklist: &Blocklist) -> Resolved;
}
```

**流程（壳侧，iOS 键盘与 Mac 输入法进程相同）**

```text
打开会话前:
  leftovers = guard.take_leftovers()
  for r in leftovers: blocklist.block(r)；记日志「包 <id> v<ver> 上次加载后崩溃，已拉黑」
  blocklist.save()
加载每个包:
  match store.resolve(id, &blocklist):
    Current(p)    => guard.begin(record_of(p)); 加载 p
    RolledBack(p) => guard.begin(record_of(p)); 加载 p; 状态条一行字「包已回退」
    Bundled/None  => 不登记；状态条「包已回退」（None 时用户包不加载）
稳定判定: 按键计数 >= STABLE_KEYS 或 打开后经过 STABLE_AFTER → guard.settle(id)
正常收起（iOS viewWillDisappear / 会话 flush，Mac 进程正常退出）: 也 settle，
  —— 否则键盘收起后被系统回收，下次会被当成崩溃（iOS 键盘扩展比 Mac 常驻进程更容易误判，这是青简要比 WeType 多做的一步）
同步拿到新版本: blocklist.release_on_new_version(id, sha) → save；新版本走正常加载与登记
下次同步请求: 把 blocklist.unreported() 带在 GET /packs 的查询里（只有 pack_id 与版本号），成功后 mark_reported
```

- 与 WeType 的差异（都是青简自己加的）：按包粒度而非整批；拉黑持久到新版本到达（WeType 5 秒后就忘）；回退到上一版（WeType 退随包）；按键计数也算稳定；上报版本号（WeType 不报）。
- 约束核对：
  - **输入优先**：`begin` / `settle` 是一次小文件写 + rename，放在加载流程里本来就不在按键路径上；`settle` 在按键路径触发时只发一个原子标志，真正写盘放到已有的 flush 时机（bridge 的 `Session::flush` / 会话关闭），不在 `qj_push` 里同步写。
  - **不删用户数据**：拉黑只影响「加载哪个文件」，文件不删；本地素材队列（Outbox）不碰。
  - **一个类型一个文件 / 目录收纳**：如上表；`load_guard.rs` 与 `guard_record.rs` 同属「守卫」概念，如果嫌前缀不齐，收成 `packs/guard/{mod,record}.rs`（contributing.md「同词干兄弟文件收进目录」）。
  - **状态条提示**：bridge 加 `qj_pack_notice(session) -> *const c_char`（无提示返回空），壳读一次画在状态条；不弹窗。
- 存储位置：iOS 有完全访问时 `<App Group>/Library/Application Support/Qingjian/packs/{loading.json,blocked.json}`；没有完全访问时扩展自己的容器（和 `UserData.resolve` 的规则一致）。Mac `~/Library/Application Support/Qingjian/packs/`。

### 4. 复刻步骤

1. 在 `qingjian-cloud-client` 加 `packs/` 模块骨架与常数，`lib.rs` re-export `LoadGuard`、`Blocklist`、`GuardRecord`。
2. 实现 `GuardRecord` / `LoadGuard`（JSON，写 `loading.json.tmp` 再 rename；读失败或解析失败当「没有残留」并删文件）。
3. 实现 `Blocklist`（JSON；`release_on_new_version` 按 pack_id 删掉 sha256 不同于新值的条目）。
4. 等 A2 的 `PackStore` 落地后实现 `resolve`（当前 → 上一版 → 底包 / 不加载）。
5. bridge：`Session::open` 前调 `take_leftovers` + 拉黑；包加载处包一层 `begin`；按键计数与计时在 `Session` 里（只记数，不写盘）；`flush` / `Drop` 时 `settle`；新增 `qj_pack_notice`。
6. iOS 壳：`KeyboardViewController.viewWillDisappear` 已有的 flush 时机确认会走到 `settle`；状态条显示 `qj_pack_notice`。Mac 壳同理（`cloud/docs/fork-patch.md` 记一笔分叉改动）。
7. 同步请求带 `unreported()`（协议字段在 `qingjian-cloud-proto` 新增，见 A2）。
8. 真机：自造一个会让加载 panic→abort 的测试包（例如把 `.qj` 头部长度字段改大，不用真实用户数据），连续调出键盘，第二次起不再崩、状态条显示「包已回退」。

**单测清单**

| 输入 | 期望 |
|---|---|
| `begin(A v2)` 后不 `settle`，新建 `LoadGuard` 调 `take_leftovers` | 返回 `[A v2]`，文件已删 |
| `begin(A v2)` → `settle("A")` → `take_leftovers` | 空 |
| `begin(A)`、`begin(B)` → `settle("A")` → `take_leftovers` | 只剩 B |
| `loading.json` 内容是乱码 | `take_leftovers` 返回空、不 panic、文件已删 |
| 残留 A v2 → 拉黑 → `resolve(A)`，store 有 v1 | `RolledBack(v1 路径)` |
| 残留 A v2 → 拉黑 → `resolve(A)`，没有上一版，A 是人设包 | `Bundled` |
| 同上，A 是用户包 | `None` |
| 拉黑 A(sha=x) 后 `release_on_new_version(A, y)` | 返回 true，`is_blocked(A,x)` 为 false |
| `release_on_new_version(A, x)`（同一版又下了一次） | 返回 false，仍拉黑 |
| 拉黑两条后 `unreported()` → `mark_reported` 一条 | 只剩另一条 |
| 按键计数到 19 | 不 settle；到 20 → settle 标志置位 |

### 5. 未知

- `dictsHash` 的折叠方式：函数 @0x10028cd64 先按 0x21 容量建 `[UInt64]`、对 md5 串取前 15 个字符位置做 `_decodeScalar`，再拼 `Data` 求 md5；具体是「每个 md5 解析成整数再异或 / 累加」还是「拼接」没读完。
  青简不需要照抄，用 `sha256(按 pack_id 排序后的 "id:version:sha256\n" 拼接)` 即可。下一步若要确认：从 0x10028ce40 读到 0x10028d0c8。
- `dictChanged` 标志（cellHash 分支里的 `w21`）在哪里置位没追，推测与「有文件缺失被跳过」有关。下一步看 doReloadDicts 0x1002971c4..0x100297210 前对 w21 的写入。
- 5 秒里崩溃之外的情况（比如用户 5 秒内正常退出）WeType 不处理，会被当成崩溃；它是 Mac 常驻进程，影响小。

---

## A2 包的增量同步

### 1. WeType 怎么做的

**角色**（全在 `WeType/Module/DictUpdate/`）[确：符号与源码路径串]

| 类 | 职责 | 关键函数 |
|---|---|---|
| `DictsUpdateChecker` | 定时触发 | `start(delayTime:)` @0x10034b328（asyncAfter），`check()` @0x10034b670 |
| `DictsUpdater` | 发请求、解析、决定下哪些 | `checkCommonDictsUpdate(specifiedDictNames:isIntervene:)` @0x10040627c，`checkNewHotDictsUpdate…` @0x100406b2c，`handleResponse(type:remoteDictItems:sourceDicts:interval:)` @0x10040779c，`clientDicts(from:specifiedDictNames:)` @0x10040979c |
| `DictsDownloader` | 串行下载队列 | `addDownload(dicts:)` @0x1003e6268，`_popAndDownloadDict()` @0x1003e65d0，`downloadPatchDict(url:dict:oldVerDict:dictItem:dictDownloadInfo:)` @0x1003e72b4（完成闭包 @0x1003e85d0），`downloadFullDict(…)` @0x1003e75ec（闭包 @0x1003e78e0），`handleFinish()` @0x1003e6d98，`failedReset()` @0x1003e82d0 |
| `DictsUtil` | 文件操作 | `unzipDict(at:)` @0x10028d240（SSZipArchive），`patchDict(basePath:patchPath:)` @0x10028d5fc，`mergeDicts` / `getLocalDicts`（读随包 `index.json`，@0x10028c83c 立即数 `index.json`） |
| `DictsManager` | 配置切换、重载、清理 | `updateConfigDicts(remoteCommonDicts:remoteNewHotDicts:)` @0x100294b20，`doReloadDicts()`（A1），`_cleanDicts()` @0x100297800 |
| `DictsConfig` | MMKV 落盘 | `loadFromMMKV()` @0x10042a7e4，`saveDictsUpdateInfo(_:key:)` |
| ObjC `BsPatchPlusBridge` + C++ `bsdiffplus::BsPatch` | 打补丁 | `+applyPatch:patchPath:patchedPath:` @0x100034a14，`BsPatch::apply` @0x100035eb8 … `mergeextra` @0x100036910 |

**index.json 字段怎么用**：随包 `imeData.bundle/index.json` 每项有 `name, md5, type, uniqueID, version, timestamp, is_cell, support_update, dict_type, special_dict_kind`（来自 wetype-reverse.md §2.1，[确：文件]）。
客户端把本地词库变成 `DictInfo`（Swift struct，Codable 键 [确：`DictInfo.CodingKeys.stringValue` @0x100162500 立即数]）：
`name, uniqueId, version(Int32), md5, typeId, timestamp, experimentGroup(Id), isLocal, dictType, isNewHot, canUpdate, path`。
`support_update` 对应 `canUpdate`（只有可更新的才进上报列表）[推：名字对应 + `clientDicts(from:)` 只组装 name/version/uniqueId/clientDictFeature]。
`md5` 用于：① 判断目标文件是否「已下载过」② 整包 / 补丁后的完整性校验 ③ A1 的集合哈希。`version` 用于：目录名、上报给服务端、清理时比较新旧。

**请求 `GetImeDictListConfigReq`**（protobuf，ObjC GPB 类，属性表 @0x1032e0f88）[确：setter selector 出现在 @0x10040627c]：
`expectedDeviceType, engineVersion, clientDictsArray[ClientDictInfo{name, version, clientDictFeature, specialDictSize, specialDictKind, uniqueId}], featureFlags, cellDictCompatibleVersion,
typeFilterArray[DictTypeFilter{dictType, compatibleVersion, engineVersion}], deviceInfo, userUsageStatus{isInitialInstallation}, clientVersion, e2EDevicePerformance, userStorageLeftInGb`。
即：**客户端报「我有哪些、各是什么版本」，由服务端决定给什么、给补丁还是整包**。

**响应 `GetImeDictListConfigResp`**（属性表 @0x1032e11e0）：`dictItemListArray, interval, underExperiment, abtestInfosArray`；客户端读了 `dictItemListArray`、`interval`、`abtestInfosArray` [确：@0x1004073b0 selector]。
`ImeDictItem`（@0x1032e08c0）客户端读的字段：`name, uniqueId, version, md5, typeId, timestamp, experimentGroupId, dictType, del, full, patch, hasFull, hasPatch` [确：selector 分布在 @0x10040779c、@0x1003e65d0、DictInfo.init(from:isNewHot:) @0x100163078]。
`ImeDictDownloadItem`（@0x1032e05f0）全部字段：`cdnURL, md5, cdnKey, decryptMd5, algorithm, compressionMd5, baseVersion, dictSizeInByte`；
下载路径只读 `cdnURL`、`md5`、`algorithm`（`== 1` 表示 zip，要先解压）[确：`cmp w22,#1` @0x1003e89a0、@0x1003e7cac]。
`cdnKey` / `decryptMd5` 只在上报函数 `BIZ.dictUpdateReportInfo` @0x100287bcc 里读 [确]；**`baseVersion` 全二进制没有 selector 引用** [确：grep `selector ref: baseVersion` 为空]——补丁的基线就是客户端报上去的那个版本，客户端不再核对。

**检查节奏**
```swift
// DictsUpdateChecker.check() @0x10034b670
let now = Date().timeIntervalSince1970
guard now - lastCheckTime >= 5.0 else { return }            // fmov d1,#5.0 @0x10034b700
if config.commonDictsUpdateInfo.nextUpdateTime < now { updater.checkCommonDictsUpdate(nil, isIntervene: false) }
if config.newHotDictsUpdateInfo.nextUpdateTime < now { updater.checkNewHotDictsUpdate(nil, isIntervene: false) }
// handleResponse 里 @0x100408448:
interval = isNewHot ? max(resp.interval, 1800) : max(resp.interval, 3600)   // 0x708 / 0xe10 秒，WeType 的值，青简要自己定
info.nextUpdateTime = now + interval
```

**handleResponse → 下载**（@0x10040779c，[推] 部分标出）
```swift
for item in remoteItems {
    if item.name.isEmpty || item.uniqueId.isEmpty { continue }        // String.count 检查
    if item.del { needDel.insert(item.name); continue }                // 日志 "[Dict][Update] handleResponse: needDelDictNames = "
    let newDict = DictInfo(from: item, isNewHot:)                      // path = dictsDownloadedPath()/<name 把 . 换成 _>/<version>/<name>
    let old = sourceDicts[name]                                        // 本地同名词库（下载的或随包的），作补丁基线 [推]
    downloads.append(DictDownloadInfo(dict: newDict, oldVerDict: old, dictItem: item))
}
saveDictsUpdateInfo(info, key: "CommonDictsUpdateInfo" | "NewHotDictsUpdateInfo")   // JSONEncoder → MMKV setData:forKey:
downloader.addDownload(downloads)
```

**单个词库的下载与补丁**（`_popAndDownloadDict` @0x1003e65d0 + 闭包 @0x1003e85d0 / @0x1003e78e0）
```swift
func popAndDownloadDict() {
    guard let info = waitingDownloadList.first else { handleFinish(); return }
    waitingDownloadList.removeFirst()
    let dst = info.dict.path
    if fileExists(dst) && WTFileMD5(dst) == info.dict.md5 {           // 0x1003e6848 / 0x1003e68a8
        log("[Dict][Download] dict[…] was downloaded before"); finishedList.append(info); popAndDownloadDict(); return
    }
    if item.hasPatch, let old = info.oldVerDict, let url = item.patch?.cdnURL, !url.isEmpty {   // 0x1003e6994..0x1003e6aac
        log("[Dict][Download] start download patchDict[…"); downloadPatchDict(url, …); return
    }
    if item.hasFull, let url = item.full?.cdnURL, !url.isEmpty {
        log("[Dict][Download] start download fullDict[…"); downloadFullDict(url, …); return
    }
    logWarn("[Dict][Download] download dict[…] failed, url is empty"); popAndDownloadDict()      // 跳过这一本
}

// downloadPatchDict 完成闭包 @0x1003e85d0（Alamofire GET 下到临时文件）
if let err = response.error { report(status: 1); failedReset(); return }
var patchPath = response.fileURL.path
if item.full?.algorithm == 1 {                                         // 注意：读的是 full.algorithm，不是 patch.algorithm [确]
    patchPath = DictsUtil.unzipDict(at: patchPath)                     // 解到 tmp/WeType/<UUID>/ 取第一个文件
    if patchPath.isEmpty { report(status: 3); failedReset(); return }
    log("[Dict][Download] did unzip patch dict …")
}
guard WTFileMD5(patchPath) == item.patch.md5 else {                    // 解压后的补丁文件 md5
    logError("… fail: patch file MD5 not match"); report(status: 3); failedReset(); return
}
let (patched, err) = DictsUtil.patchDict(basePath: old.path, patchPath: patchPath)   // old.path = DictInfo+0x58
guard !patched.isEmpty, WTFileMD5(patched) == item.md5 else {          // 打完后对的是 ImeDictItem.md5（最终文件）
    logError("… fail: patched file MD5 not match"); report(status: 4, patchErrCode: err); failedReset(); return
}
do { try FileManager.default.moveItem(atPath: patched, toPath: dst) }
catch { logError("… fail: move file error = …"); report(status: 1); failedReset(); return }
log("[Dict][Download] download and patch dict …"); report(status: 0); finishedList.append(info); popAndDownloadDict()

// downloadFullDict 闭包同构：下载 → algorithm==1 则 unzip → md5 == item(full).md5 → move → 下一本；任一失败 failedReset()
```

`failedReset()` 的闭包 @0x1003e9b4c [确]：`waitingDownloadList = []; finishedList = []; isDownloading = false`。即**一本失败整批作废**，配置不切换；
**没有「补丁失败当场改下整包」**的代码路径 [确：完成闭包里失败分支只调 failedReset]。下次检查时服务端可能再给补丁或整包，由服务端决定。

**DictsUtil.patchDict(basePath:patchPath:)** @0x10028d5fc [确]
```swift
let dir = tmpPath() + "/WeType/patched"          // 立即数 "/WeType/" + "patched"
generateDirectory(dir)
let out = dir + "/" + UUID().uuidString
if fileExists(out) { try removeItem(out) }
FileManager.default.createFile(atPath: out, contents: Data())
let rc = BsPatchPlusBridge.applyPatch(basePath, patchPath: patchPath, patchedPath: out)
return (rc == BsPatchPlusBridge.bsPatchErrOK && fileExists(out)) ? (out, rc) : ("", rc)   // bsPatchErrOK 恒为 0（@0x100034a0c）
```

**BsPatch 格式**（@0x100035ef8 openFiles、@0x100036510 bspatch）[确]
- 调用约定：`BsPatch::apply(oldPath, newPath, patchPath)`，与经典 `bspatch oldfile newfile patchfile` 同序；ObjC 层把参数重排过（@0x100034aa4）。
- 头 32 字节：`"BSDIFF40"`（`0x3034464649445342` 比较 @0x10003600c）+ 三个 8 字节 sign-magnitude 整数 `ctrllen, difflen, newsize`（offtin 解码 @0x10003603c..0x100036130），任何一个为负报 `head info incorrect…`。
- 三段各自 `fopen` + `fseeko(32 / 32+ctrllen / 32+ctrllen+difflen)` + `BZ2_bzReadOpen`：**标准 bzip2 压缩的 BSDIFF40**，不是 zstd / 无压缩变体。
- 「plus」之处：old 用 `open/read` 顺序读、`lseek(SEEK_CUR, ctrl[2])` 跳，new 用 `write` 顺序写，块大小 0x2000（8 KB），**不把 old / new 整个读进内存**——流式低内存版。数据格式与 bsdiff 4.x 完全兼容，任何标准 bsdiff 生成的补丁都能打。
- 错误码：-1019 打不开补丁、-1018 读头失败/读 old 失败、-1017 seek、-1016 写失败、-1015 关文件、-1014 头不对、-1013 bz 打开/读 diff 失败（WeType 的值，只用于上报）。

**目标目录与「指针」**
- 路径 [确：`DictInfo.init(from:isNewHot:)` @0x1001631d8..0x1001633e0]：`dictsDownloadedPath()/<name.replacing(".", "_")>/<version>/<name>`，
  本机实例如 `DictUpdate/DownloadDicts/wsd_bin/1045/wsd.bin`、`cell_dict_fi/763/cell_dict.fi`（目录名观察，未读内容）。
- **没有文件系统指针**：当前用哪个版本记在 MMKV 的 `CommonDictsUpdateInfo` / `NewHotDictsUpdateInfo`（`DictsUpdateInfo` 的 JSON，Codable 键 [确 @0x1003fdbb8]：
  `kind, nextUpdateTime, engineVersion, cellDictVersion, wubiDictVersion, opt_largeDictVersion, opt_e2eDictVersion, opt_baseDictVersion, opt_secondLevelDictVersion, opt_nnlmDictVersion, opt_jianpinDictVersion, opt_handwritingDictVersion, dicts`）。
  `dicts: [DictInfo]` 里每项带完整 `path`。切换 = 全部下载成功后 `handleFinish` → `DictsManager.updateConfigDicts(remoteCommonDicts:remoteNewHotDicts:)` 写回 MMKV
  （日志 `[Dict][Main] all dicts download finished. update config dicts`）→ 发通知 → `doReloadDicts()`。整批原子：要么全切、要么（failedReset）都不切。
- `DictsUpdateInfo.isEngineVerMatched()` @0x1003fd6f8 拿 `engineVersion` 与 `WXIMEDef.wximeShortVersion` 比，`availableDicts()` @0x1003fc59c 再按各 `*DictVersion` 与文件存在过滤——App 升级后旧下载的不兼容词库自动不用 [推]。

**旧版本保留与清理**（`_cleanDicts()` @0x100297800，由 `AppDelegate.launchInitServices()` 经 `cleanDicts()` 在启动时调 [确：调用点 0x10015aefc / 0x10011b45c]）
```swift
let inUse = Dictionary(uniqueKeysWithValues: (common.dicts + newHot.dicts).map { ($0.name.replacing(".", "_"), $0) })
for folder in contentsOfDirectory(dictsDownloadedPath()) {
    guard let d = inUse[folder] else { removeItem(folder); log("[Dict][Main] remove dicts at folder path: …"); continue }
    for sub in contentsOfDirectory(folder) {
        if let v = Int32(sub), d.version <= v { continue }      // cmp w8,w0; b.le @0x1002983a4：当前版本和更新的留下
        removeItem(folder + "/" + sub); log("[Dict][Main] remove dict at path: …")
    }
}
```
即只留当前在用版本（以及可能正在下载的更新版本），**不留上一版**；回退目标是随包数据。

**上报**：`BIZ.reportDictUpdate(_:patchVer:downloadTime:handleTime:updateStatus:patchErrCode:)` @0x1002882c4，字段 `dictName, version, patchVersion, dictMdfive, cdnURL, patchCdnURL, decryptMdfive, cdnMdfive, compressionAlgorithm, decryptAlgorithm, dictType, typeId, experimentGroupId, engineVersion, wximeVersion, apiLevel, downloadTime, handleTime, updateStatus, patchErrorCode` [确：setter selector]。
updateStatus：0 成功、1 下载或移动失败、3 解压失败 / 补丁 md5 不符、4 打补丁后 md5 不符（带 BsPatch 错误码）[推：按各失败分支传的 w2 常数]。

### 2. 证据表

| 结论 | 置信度 | 地址 / 字符串 |
|---|---|---|
| 补丁格式是标准 BSDIFF40 + bzip2 | 确 | 魔数比较 @0x10003600c；`BZ2_bzReadOpen` @0x10003625c/0x100036310/0x1000363bc |
| 流式打补丁，8 KB 缓冲 | 确 | 构造 @0x100035e4c `malloc(0x2000)`×2；mergediff `read`/`write` @0x100036828/0x10003686c；`lseek SEEK_CUR` @0x1000365b8 |
| apply(old, new, patch) | 确 | 0x100034aa4..0x100034aac 参数排布 |
| 先下补丁，条件 hasPatch 且有本地旧版且 URL 非空 | 确 | @0x1003e6994、0x1003e69a0、0x1003e6aac |
| 补丁文件 md5 校验 → 打补丁 → 结果 md5 校验 → move | 确 | @0x1003e8af8、0x1003e8ba8、0x1003e8c0c、0x1003e8d90 |
| algorithm==1 → 先 unzip | 确 | `cmp w22,#1` @0x1003e89a0；`unzipDict(at:)` 用 SSZipArchive |
| 失败整批中止，不当场退整包 | 确 | failedReset 闭包 @0x1003e9b4c 清两个列表 |
| 已下载且 md5 对得上就跳过 | 确 | `WTFileMD5` @0x1003e6848；日志 `] was downloaded before` |
| 版本目录 `<name . → _>/<version>/<name>` | 确 | @0x1001631f8（`.`）/0x100163200（`_`）+ `Int32.description` |
| 「指针」是 MMKV 里的 JSON 配置，不是文件 | 确 | `CommonDictsUpdateInfo` / `NewHotDictsUpdateInfo` + `setData:forKey:` @0x100408a4c |
| 只保留当前及更新版本 | 确 | `_cleanDicts` @0x1002983a4 比较 |
| baseVersion / cdnKey / decryptMd5 不参与下载 | 确 | 全文件 selector 引用只在上报函数 |
| 检查间隔下限 1800 / 3600 秒 | 确（值） / 推（哪个对应 newHot） | @0x100408448..0x100408470 |
| 两次检查最短间隔 5 秒 | 确 | @0x10034b700 |
| 客户端上报本地清单，服务端决定补丁 / 整包 | 推 | 请求里有 clientDictsArray，响应 item 同时可带 full 与 patch |

### 3. 青简落点

**现状**：
- `crates/qingjian-update`（上游 crate，分叉补丁已改成自建服务器）只做「读签名的 releases.json → 下安装包到 `updates/` → sha256 对上才留」（`src/download.rs` `fetch_package`），是**整包、App 级**的，不管数据包。签名校验 `src/index/signature.rs::verify` 可以复用思路，但人设包 / 用户包走的是带账号鉴权的 `/packs` 接口，不走公开索引，所以**不建议把包同步塞进 `qingjian-update`**（会把账号依赖带进上游 crate，违反分叉隔离）。
- `cloud/crates/qingjian-cloud-client`：已有 `Client`（ureq，阻塞）、`supervise` 退避、`DataSync` 后台线程（`data_sync/mod.rs`：30 秒一轮、退避 1 秒起到 5 分钟、401/403 处理）。包同步应作为 `DataSync` 里的一个新 job（spec 第 85 行：启动、批处理完成推送、兜底每 6 小时查一次版本）。
- `cloud/crates/qingjian-cloud-proto`：一类型一文件的协议类型（`learning_page.rs` 之类），新增 `/packs` 的类型放这里。
- 两个工作区的 Cargo.lock 里目前都没有 bzip2 / bsdiff 类依赖（只有 `zip`、`flate2`）。
- 加密：`cloud/docs/specs/sujian-encryption.md` 第 35 行字段加密是「AES-256-GCM、每次随机 nonce」，第 79 行本机包用设备密钥加密落盘。

**和任务原文的冲突与改法**
1. 「补丁对密文做」：如果整包用 AES-GCM + 随机 nonce 加密，两版密文完全不同，bsdiff 补丁≈整包大小，增量失去意义。两种改法任选其一，建议 (a)：
   - (a) 服务端本来就要明文学习、打包（spec「云端学习」），所以**对明文求 bsdiff，再把补丁本身用同一把 DEK 加密下发**；客户端解密补丁 → 对本地明文旧版打补丁 → 校验明文 sha256 → 用设备密钥重新加密落盘。
   - (b) 包改成按 64 KB 分块、nonce 由块内容派生（AES-GCM-SIV），密文才有可比性；改动大，不推荐。
   协议里的 `sha256` 一律指**明文**的 sha256（和 A1 拉黑用同一个值），密文完整性靠 GCM 标签。
2. 「iOS / Mac 共用 Rust 侧打补丁」：对，但**iOS 上打补丁放在主 App（或后台任务）里做，不在键盘扩展里做**——扩展内存上限低（spec 与 README 都提了 jetsam），解密 + 补丁要同时持有旧版明文与新版。键盘扩展只读 `current.json` 指向的文件。
3. 「失败退整包」：WeType 没有这条（整批中止等下次）。青简照任务原文做「当场退整包」更好，成本只是多一次请求。
4. 「保留上一版」：WeType 不留；青简要留（A1 回退要用），清理规则改成「留 current 与 previous，其余删」。

**选库**：`qbsdiff`（BSDIFF40 + bzip2，与 WeType 用的格式一致；许可证 MIT，与 GPL-3.0-or-later 兼容——加依赖前用 `cargo add` 后再核一次 crate 元数据）。
它的 bzip2 后端要选纯 Rust 的（`bzip2` crate 的 `libbz2-rs-sys` 后端），避免给 iOS 交叉编译 C 库；加之前在 `cloud/` 下 `cargo check --target aarch64-apple-ios` 验一次。
`qbsdiff::Bspatch` 要求旧版整块在内存（`&[u8]`），明文包是 1–5 MB 量级（spec 第 94 行），在主 App 里可以接受；不需要复刻 WeType 的流式版。
服务端（synon-ime，闭源仓库）用同一个 crate 的 `Bsdiff` 生成，保证两端格式一致。

**新增文件**

`cloud/crates/qingjian-cloud-proto/src/`
| 文件 | 内容 |
|---|---|
| `pack_list.rs` | `PackList { packs: Vec<PackOffer>, interval_secs: u32 }`（`GET /packs` 的响应） |
| `pack_offer.rs` | `PackOffer { id: String, kind: PackKind, version: u64, sha256: String, size: u64, full_url: String, patch: Option<PackPatch>, deleted: bool }` |
| `pack_patch.rs` | `PackPatch { from_version: u64, url: String, sha256: String /* 解密后补丁本身的 sha256 */, size: u64 }` |
| `pack_kind.rs` | `enum PackKind { Persona, Contact }` |
| `pack_local.rs` | `PackLocal { id: String, version: u64 }`（请求里报本地清单；`GET /packs?have=id:ver,…&crashed=id:ver,…`，crashed 来自 A1 `Blocklist::unreported`） |

`cloud/crates/qingjian-cloud-client/src/packs/`（与 A1 同目录）
| 文件 | 内容 |
|---|---|
| `pack_store.rs` | `PackStore`：目录布局、`current` / `previous`、原子切换、清理 |
| `current.rs` | `Current { version: u64, sha256: String, previous: Option<(u64, String)> }`，存 `<id>/current.json` |
| `patch.rs` | 自由函数 `apply_patch(old: &[u8], patch: &[u8]) -> Result<Vec<u8>, PackError>`（包 qbsdiff） |
| `pack_sync.rs` | `PackSync`：一轮同步（列表 → 逐包补丁 / 整包 → 校验 → 落盘 → 切换） |
| `pack_error.rs` | `PackError`（thiserror，`#[error]` 英文） |

**目录与指针**
```text
<packs_dir>/                         iOS: <App Group>/Library/Application Support/Qingjian/packs/；Mac: ~/Library/Application Support/Qingjian/packs/
  <pack_id>/
    current.json                     {"version":7,"sha256":"…","previous":[6,"…"]}，写 current.json.tmp 再 rename = 原子切换
    7/pack.qjp                       设备密钥加密后的明文包
    6/pack.qjp                       上一版，留着给 A1 回退
  loading.json / blocked.json        A1
```
WeType 用「配置里记版本 + 版本目录」，青简同构，只是配置换成每包一个 `current.json`（键盘扩展与主 App 跨进程读，用文件比 MMKV 简单）。

**签名**
```rust
impl PackStore {
    pub fn open(dir: &Path) -> Self;
    pub fn current(&self, id: &str) -> Option<Current>;
    pub fn path_of(&self, id: &str, version: u64) -> PathBuf;              // <id>/<version>/pack.qjp
    /// 新版本文件已写好并校验过：先 fsync 文件，再写 current.json.tmp → rename；旧 current 变 previous。
    pub fn commit(&self, id: &str, version: u64, sha256: &str) -> io::Result<()>;
    /// 只留 current 与 previous 两个版本目录；被拉黑的版本不特殊对待（文件留着无害，下次 commit 后自然被清）。
    pub fn prune(&self, id: &str) -> io::Result<()>;
    pub fn remove(&self, id: &str) -> io::Result<()>;                      // 服务端 deleted=true
    pub fn resolve(&self, id: &str, blocklist: &Blocklist) -> Resolved;   // A1
}

impl PackSync {
    pub fn new(client: Client, store: PackStore, keys: Box<dyn LocalCipher>) -> Self;
    /// 一轮：GET /packs 带本地清单与崩溃上报；逐个处理 offer；返回下次间隔。不在键盘扩展里调。
    pub fn run_once(&mut self) -> Result<Duration, PackError>;
}

/// 本机加解密由壳注入（iOS Keychain / Mac 钥匙串），client crate 不碰平台 API。
pub trait LocalCipher: Send {
    fn seal(&self, plain: &[u8]) -> Result<Vec<u8>, PackError>;
    fn open(&self, sealed: &[u8]) -> Result<Vec<u8>, PackError>;
}
```

**单个包的处理（青简版伪代码）**
```rust
fn sync_one(&mut self, offer: &PackOffer) -> Result<(), PackError> {
    if offer.deleted { return self.store.remove(&offer.id).map_err(Into::into); }
    let cur = self.store.current(&offer.id);
    if cur.as_ref().is_some_and(|c| c.sha256 == offer.sha256) { return Ok(()); }        // 同 WeType「downloaded before」
    let target = self.store.path_of(&offer.id, offer.version);
    if target.is_file() && sha256(self.keys.open(&read(&target)?)?) == offer.sha256 {  // 上次写好了但没切
        return self.finish(offer);
    }
    let plain = match (&offer.patch, &cur) {
        (Some(p), Some(c)) if p.from_version == c.version => {
            match self.try_patch(offer, p, c) {                                          // 下补丁 → 解 DEK → 校 p.sha256 → apply → 校 offer.sha256
                Ok(bytes) => bytes,
                Err(e) => { tracing::warn!(%e, id = %offer.id, "补丁失败，改下整包"); self.download_full(offer)? }
            }
        }
        _ => self.download_full(offer)?,                                                // 没有补丁或本地版本对不上
    };
    if sha256(&plain) != offer.sha256 { return Err(PackError::ChecksumMismatch); }      // 整包也校验；失败不切，这一个包等下一轮
    write_new(&target, &self.keys.seal(&plain)?)?;                                       // 写 .part → fsync → rename
    self.finish(offer)
}
fn finish(&mut self, offer: &PackOffer) -> Result<(), PackError> {
    self.store.commit(&offer.id, offer.version, &offer.sha256)?;
    self.blocklist.release_on_new_version(&offer.id, &offer.sha256);                    // A1：新版本到达解除拉黑
    self.store.prune(&offer.id)?;
    Ok(())
}
```
- 与 WeType 的不同：按包独立成败（WeType 一本失败整批作废）；补丁失败当场退整包；sha256 替代 md5。
- 间隔：服务端给 `interval_secs`，客户端取 `max(interval_secs, 6 * 3600)` 作兜底轮询（spec 第 85 行「每 6 小时」），WeType 的 1800 / 3600 不照搬；另有推送触发。
- 上限：沿用 `qingjian-update` 的做法给单包与补丁设字节上限（例如 64 MB），超了丢弃。

**服务端（synon-ime，这里只写接口约定）**：每次打包保留最近 K=3 版明文；请求带 `have=id:ver` 时，若 `ver` 在保留窗口内就给 `patch{from_version=ver}`（缓存 `(from,to)` 的 bsdiff 结果），否则只给 `full_url`；补丁与整包都用该用户该用途的 DEK 加密下发。

### 4. 复刻步骤

1. `cloud/` 下 `cargo add qbsdiff -p qingjian-cloud-client`（核许可与 bzip2 后端；跑 `cargo check --target aarch64-apple-ios`）。
2. `qingjian-cloud-proto` 加 `pack_list / pack_offer / pack_patch / pack_kind / pack_local`，serde 往返单测。
3. `packs/patch.rs`：`apply_patch` 包 qbsdiff，拒绝非 `BSDIFF40` 头（返回 `PackError::BadPatch`），输出长度超上限报错。
4. `packs/current.rs` + `pack_store.rs`：目录布局、`commit` 原子切换（tmp + rename）、`prune` 留两版、`resolve`。
5. `packs/pack_sync.rs`：`run_once` 与 `sync_one`，通过 `test_support` 假服务覆盖补丁 / 整包 / 失败回退。
6. 接进 `DataSync` 的 jobs（`data_sync/jobs.rs`），只在主 App 与 Mac 进程里启动；iOS 键盘扩展不启动。
7. bridge 的 `Session::open` 改为经 `PackStore::resolve` 取包路径（与 A1 一起），解密由壳注入的 `LocalCipher`。
8. 服务端（synon-ime）按上节约定实现，补丁用同一 crate 生成。
9. 记一笔分叉改动到 `cloud/docs/fork-patch.md`；用户可感知的「包已回退」提示写进对应用户文档（若该提示进上游壳，要改 `docs/user/`）。

**单测清单**

| 输入 | 期望 |
|---|---|
| 合成三版 v1/v2/v3（随机 1 MB，v2/v3 各改 5%），`apply(apply(v1, d12), d23)` | 与 v3 字节一致；记录 `len(d12)/len(v2)`、`len(d23)/len(v3)` |
| 同一组数据直接整包 | 与上一条结果字节一致 |
| 补丁头被改成 `BSDIFF41` | `PackError::BadPatch`，store 不变 |
| 补丁正确但 offer.sha256 写错 | 先退整包；整包 sha 也对不上 → `ChecksumMismatch`，current 不变 |
| offer.patch.from_version=2，本地 current=1 | 不下补丁，直接整包 |
| 本地无 current，offer 带 patch | 整包 |
| `commit(A,3)` 后 `current(A)` | version=3，previous=(2,…) |
| `prune(A)` 时磁盘有 1/2/3 | 只剩 2、3 |
| `commit` 写 current.json.tmp 后模拟 rename 前中断（直接留下 tmp） | `current(A)` 仍读到旧值；下次 `commit` 覆盖 tmp |
| target 文件已在且 sha 对 | 不发下载请求，直接 commit |
| offer.deleted=true | 目录删除，current 消失 |
| A1 已拉黑 A(sha=x)，同步到 A(sha=y) | 拉黑解除，resolve 返回 Current(y) |
| 服务端（synon-ime）：保留窗口外的 from | 响应只有 full_url |

### 5. 未知

- `handleResponse` 里「哪些远端条目需要下载」的完整判定（是否还对比本地 md5 / version）只读到 `stringCompare` @0x1004081b8 与对 `sourceDicts` 的查找，没完全展开；不影响青简（青简按 sha256 判）。下一步读 0x100407e00..0x1004081f0。
- `interval` 的两个下限分别对应 common 还是 newHot：分支条件是 `[x19+0x148]`（handleResponse 的 `type` 参数）是否为 0，没核实 0 代表哪一类。
- 补丁的 zip 判断读 `full.algorithm` 而不是 `patch.algorithm`，是 WeType 的写法还是我读错寄存器，已核对 `x26` = dictItem、selector `full` @0x1003e8974，倾向是它本来就这么写（服务端两者一致）；对青简无影响。
- `DictsUtil.unzipDict(at:)` 取解压目录里的哪个文件（第一个？按名字？）没细读（@0x10028d240）。
- 服务端如何选补丁基线（是否只给最近若干版）在客户端看不到。
