// 青简引擎的 Swift 包装：持有 Rust 桥的会话指针，把 C 字符串转成 String。只在主线程上用。

import Foundation
import QingjianBridge

@MainActor
final class Engine {
    /// deinit 不在主线程隔离里，要绕开 Sendable 检查；释放时已没有别的引用。
    private nonisolated(unsafe) let session: OpaquePointer

    /// `dataDirectory` 里要有 dict.qj（lm.qj 可选）；`userDirectory` 放学习数据。打不开返回 nil。
    init?(dataDirectory: URL, userDirectory: URL?) {
        let opened = dataDirectory.path.withCString { data in
            if let user = userDirectory?.path {
                return user.withCString { qj_session_open(data, $0) }
            }
            return qj_session_open(data, nil)
        }
        guard let opened else { return nil }
        session = opened
    }

    deinit {
        qj_session_free(session)
    }

    var composing: Bool { qj_composing(session) }

    var preedit: String { take(qj_preedit(session)) ?? "" }

    var candidates: [String] {
        (0..<qj_candidate_count(session)).compactMap { take(qj_candidate_text(session, $0)) }
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

    func flush() { qj_flush(session) }

    private func take(_ raw: UnsafeMutablePointer<CChar>?) -> String? {
        guard let raw else { return nil }
        defer { qj_string_free(raw) }
        return String(cString: raw)
    }
}
