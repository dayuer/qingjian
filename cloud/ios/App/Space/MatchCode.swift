// 用户输入的匹配码：去掉空白与连字符、转大写，够不够 8 位。
// 只做这一层，`I/L→1`、`O→0` 那些 Crockford 的规范化放在服务端（proto 的 `normalize_pair_code`），不做第二份。

import Foundation

enum MatchCode {
    /// 匹配码的位数，与 `qingjian_cloud_proto::PAIR_CODE_LEN` 一致。
    static let length = 8

    /// 去掉空白与连字符、转成大写；再长的原样留着（让服务端去判，本地裁掉会掩盖粘错东西）。
    static func entered(_ raw: String) -> String {
        raw.filter { !$0.isWhitespace && $0 != "-" }.uppercased()
    }

    static func isComplete(_ raw: String) -> Bool {
        entered(raw).count == length
    }

    /// 还差几位；输多了是 0，不是负数。
    static func remaining(_ raw: String) -> Int {
        max(0, length - entered(raw).count)
    }
}
