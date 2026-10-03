// 青简引擎的 Swift 包装：持有 Rust 桥的会话指针，把 C 字符串转成 String。只在主线程上用。

import Foundation
import QingjianBridge

@MainActor
final class Engine {
    /// deinit 不在主线程隔离里，要绕开 Sendable 检查；释放时已没有别的引用。
    private nonisolated(unsafe) let session: OpaquePointer

    /// `dataDirectory` 里要有 dict.qj（lm.qj 可选）；`userDirectory` 放学习数据；
    /// `cloudConfig` 指向 cloud.toml，没有就完全离线。打不开返回 nil。
    init?(dataDirectory: URL, userDirectory: URL?, cloudConfig: URL?) {
        let opened = Self.withOptionalCString(userDirectory?.path) { user in
            Self.withOptionalCString(cloudConfig?.path) { cloud in
                dataDirectory.path.withCString { qj_session_open($0, user, cloud) }
            }
        }
        guard let opened else { return nil }
        session = opened
    }

    deinit {
        qj_session_free(session)
    }

    var composing: Bool { qj_composing(session) }

    var preedit: String { take(qj_preedit(session)) ?? "" }

    var candidates: [CandidateItem] {
        (0..<qj_candidate_count(session)).compactMap { index in
            take(qj_candidate_text(session, index)).map {
                CandidateItem(text: $0, cloud: qj_candidate_is_cloud(session, index))
            }
        }
    }

    func push(_ letter: Character) {
        guard let scalar = letter.unicodeScalars.first else { return }
        qj_push(session, scalar.value)
    }

    func backspace() { qj_backspace(session) }

    func clear() { qj_clear(session) }

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

    func startRewrite(_ text: String) {
        text.withCString { qj_rewrite_start(session, $0) }
    }

    /// 0 空闲、1 等待中、2 就绪、3 失败（与桥的约定一致）。
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

    private func take(_ raw: UnsafeMutablePointer<CChar>?) -> String? {
        guard let raw else { return nil }
        defer { qj_string_free(raw) }
        return String(cString: raw)
    }

    private static func withOptionalCString<T>(
        _ string: String?, _ body: (UnsafePointer<CChar>?) -> T
    ) -> T {
        guard let string else { return body(nil) }
        return string.withCString { body($0) }
    }
}
