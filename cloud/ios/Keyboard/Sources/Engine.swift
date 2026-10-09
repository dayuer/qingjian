// 素笺引擎的 Swift 包装：持有 Rust 桥的会话指针，把 C 字符串转成 String。只在主线程上用。

import Foundation
import QingjianBridge

@MainActor
final class Engine {
    /// deinit 不在主线程隔离里，要绕开 Sendable 检查；释放时已没有别的引用。MemoryBridge.swift 的扩展也用它。
    nonisolated(unsafe) let session: OpaquePointer

    /// 随包产品数据目录（里面的 dicts/ 是领域词库）。
    let dataDirectory: URL

    /// 学习数据与记忆的目录（App Group 的 `Qingjian`）；`qj_memory_*` 按目录传时用。没开完全访问时是扩展自己的容器。
    let userDirectory: URL?

    /// `dataDirectory` 里要有 dict.qj（lm.qj 可选）；`userDirectory` 放学习数据；`configFile` 是设置（config.toml，
    /// 变了轮询时自动重读）；`cloudConfig` 指向 cloud.toml，没有就完全离线。打不开返回 nil。
    init?(dataDirectory: URL, userDirectory: URL?, configFile: URL?, cloudConfig: URL?) {
        self.dataDirectory = dataDirectory
        self.userDirectory = userDirectory
        let opened = Self.withOptionalCString(userDirectory?.path) { user in
            Self.withOptionalCString(configFile?.path) { config in
                Self.withOptionalCString(cloudConfig?.path) { cloud in
                    dataDirectory.path.withCString { qj_session_open($0, user, config, cloud) }
                }
            }
        }
        guard let opened else { return nil }
        session = opened
        LiveCount.created("engine")
    }

    /// 后台打分算完时在主线程上调的处理（见 `onRescored`）。
    private var rescoreHandler: (() -> Void)?

    /// 回调上下文：弱引用引擎的盒子，交给桥按地址带过线程；deinit 先清回调再放它。
    nonisolated(unsafe) private var rescoreBox: Unmanaged<WeakEngine>?

    deinit {
        LiveCount.released("engine")
        qj_set_rescore_notify(session, nil, nil)
        rescoreBox?.release()
        qj_session_free(session)
    }

    /// 最近一次整句重排的分段耗时（`queue=…;forward=…;main=…;paths=…;max_chars=…`），真机验收的日志用。
    var rescoreStats: String? { take(qj_rescore_stats(session)) }

    /// 停键多久送整句路径去重排（桥给的数，与 Mac 壳一致）。
    static var rescoreDebounce: TimeInterval { Double(qj_rescore_debounce_ms()) / 1000 }

    /// 重排一拍：停键 `rescoreDebounce` 后调一次送去打分，`onRescored` 的处理里再调一次取结果；候选栏要重画返回 true。
    func rescoreTick() -> Bool { qj_rescore_tick(session) }

    /// 本地整句模型打完分时在主线程上调 `handler`（打分线程算完就通知，不等 250ms 的轮询）。
    func onRescored(_ handler: @escaping () -> Void) {
        rescoreHandler = handler
        guard rescoreBox == nil else { return }
        let box = Unmanaged.passRetained(WeakEngine(self))
        rescoreBox = box
        qj_set_rescore_notify(session, rescoreNotified, box.toOpaque())
    }

    /// 回到主线程后真正处理通知（`rescoreNotified` 排过来）。
    fileprivate func handleRescored() { rescoreHandler?() }

    var composing: Bool { qj_composing(session) }

    var preedit: String { take(qj_preedit(session)) ?? "" }

    /// 一次过桥取回整栏（格式见 qj_candidates），每键只分配一次。
    var candidates: [CandidateItem] {
        guard let joined = take(qj_candidates(session)), !joined.isEmpty else { return [] }
        return joined.split(separator: "\u{1e}", omittingEmptySubsequences: false).compactMap { cell in
            guard let flag = cell.first else { return nil }
            return CandidateItem(text: String(cell.dropFirst()), cloud: flag == "1")
        }
    }

    func push(_ letter: Character) {
        guard let scalar = letter.unicodeScalars.first else { return }
        qj_push(session, scalar.value)
    }

    func backspace() { qj_backspace(session) }

    func clear() { qj_clear(session) }

    // MARK: 内存自保

    /// 本进程剩余可用内存（MB）；拿不到是 -1。键盘的内存自保用它。
    static var availableMemoryMB: Double { qj_available_memory_mb() }

    /// 卸下英文词表；下次像英文的输入自动再加载（词表是 mmap 的，卸掉主要是让出地址空间与页缓存）。
    func unloadEnglish() { qj_unload_english(session) }

    /// 随包的 8 位含章·通变（`Data/models/hanzhang-tongbian-q8.qjm`）在后台加载，接上后整句候选带神经重排
    /// （只重排词图的整句路径，不自由生成）。权重是 mmap 的干净页，加载只多约 1MB dirty。没有这个文件返回 false。
    @discardableResult
    func loadModel() -> Bool {
        let url = dataDirectory.appendingPathComponent("models/hanzhang-tongbian-q8.qjm")
        guard FileManager.default.fileExists(atPath: url.path) else { return false }
        return url.path.withCString { qj_load_model(session, $0, true) }
    }

    /// 本地整句模型的状态：0 未加载 / 1 加载中 / 2 在用 / 3 上次失败。
    var modelState: UInt8 { qj_model_state(session) }

    /// 最近一次模型加载的报告（`ok|failed sha=… ms=…[ error=…]`），还没出结果是 nil。
    var modelReport: String? { take(qj_model_report(session)) }

    /// 卸下本地整句模型（内存吃紧时）；之后可以再 `loadModel()`。
    func unloadModel() { qj_unload_model(session) }

    /// 上屏第 `index` 个候选，返回要插入的文字。
    func commit(_ index: Int) -> String? {
        guard index >= 0 else { return nil }
        return take(qj_commit(session, UInt32(index)))
    }

    func takeRaw() -> String { take(qj_take_raw(session)) ?? "" }

    /// 没在组句时敲的标点，返回要插入的文字（中文模式转全角）。
    func punctuate(_ mark: Character) -> String {
        guard let scalar = mark.unicodeScalars.first,
              let text = take(qj_punctuate(session, scalar.value))
        else { return String(mark) }
        return text
    }

    /// 没在组字时直接输出的空格、回车，记进输入日志的文本流。
    func notePassthrough(_ c: Character) {
        guard let scalar = c.unicodeScalars.first else { return }
        qj_note_passthrough(session, scalar.value)
    }

    func flush() { qj_flush(session) }

    var cloudEnabled: Bool { qj_cloud_enabled(session) }

    var rewriteAvailable: Bool { qj_rewrite_available(session) }

    func setContext(before: String, after: String) {
        before.withCString { b in after.withCString { qj_set_context(session, b, $0) } }
    }

    /// 合并别的设备的学习数据、取回大模型结果；候选变了返回 true。
    func poll() -> Bool { qj_poll(session) }

    func syncNow() { qj_sync_now(session) }

    /// 开始改写；`skillId` 为空时用桥那边当前生效的那个（设置里的默认 → 列表第一个）。
    func startRewrite(_ text: String, skillId: String?) {
        text.withCString { t in
            Self.withOptionalCString(skillId) { qj_rewrite_start(session, t, $0) }
        }
    }

    /// 0 空闲、1 等待中、2 就绪、3 失败、4 模型给的不合用（已丢掉）（与桥的约定一致）。
    var rewriteStatus: UInt32 { qj_rewrite_status(session) }

    func takeRewrite() -> String? { take(qj_rewrite_take(session)) }

    func cancelRewrite() { qj_rewrite_cancel(session) }

    /// 验证码、密码、信用卡号这类输入框：不学习、不记日志、不发云端，剪贴板与润色也停。
    func setPrivate(_ value: Bool) { qj_set_private(session, value) }

    var clipboardEnabled: Bool { qj_clipboard_enabled(session) }

    /// 后台拉一次别的设备的剪贴板（键盘弹出时调）。
    func refreshClipboard() { qj_clip_refresh(session) }

    /// 别的设备最近复制、还没处理过的文字。
    var clipOffer: ClipOffer? {
        guard let text = take(qj_clip_offer_text(session)) else { return nil }
        return ClipOffer(device: take(qj_clip_offer_device(session)) ?? "", text: text)
    }

    func clipHandled() { qj_clip_handled(session) }

    func pushClip(_ text: String) {
        text.withCString { qj_clip_push(session, $0) }
    }

    func take(_ raw: UnsafeMutablePointer<CChar>?) -> String? {
        guard let raw else { return nil }
        defer { qj_string_free(raw) }
        return String(cString: raw)
    }

    static func withOptionalCString<T>(
        _ string: String?, _ body: (UnsafePointer<CChar>?) -> T
    ) -> T {
        guard let string else { return body(nil) }
        return string.withCString { body($0) }
    }
}

/// 桥在**打分线程**上调的 C 回调。必须是 nonisolated 的顶层函数：写成 `@MainActor` 方法里的闭包会被推断成主线程隔离，
/// Swift 在后台线程上调它时做隔离检查直接 trap（模拟器浸泡测出的崩溃，栈顶 `_dispatch_assert_queue_fail`）。
private nonisolated func rescoreNotified(_ context: UnsafeMutableRawPointer?) {
    guard let context else { return }
    let box = Unmanaged<WeakEngine>.fromOpaque(context).takeUnretainedValue()
    DispatchQueue.main.async {
        MainActor.assumeIsolated { box.engine?.handleRescored() }
    }
}

/// 桥的回调上下文：只弱引用引擎，引擎没了回调就什么也不做。
final class WeakEngine: @unchecked Sendable {
    weak var engine: Engine?

    init(_ engine: Engine) { self.engine = engine }
}
