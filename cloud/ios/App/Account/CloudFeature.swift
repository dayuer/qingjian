// 要用户单独打开的云功能；rawValue 是协议里的名字（交给桥的 qj_account_set_consent）。

import Foundation

enum CloudFeature: String, CaseIterable, Identifiable {
    case clipboard
    case sync
    case inputLog = "input_log"
    case llm
    case memory

    var id: String { rawValue }

    var title: String {
        switch self {
        case .clipboard: "跨设备剪贴板"
        case .sync: "同步学习数据与设置"
        case .inputLog: "上传输入日志（服务器据此纠错调频）"
        case .llm: "大模型（润色、云联想）"
        case .memory: "云端记忆（把记下的素材整理成卡）"
        }
    }
}
