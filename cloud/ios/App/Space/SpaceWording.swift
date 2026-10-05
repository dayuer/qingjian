// 开通页与输码页上的全部文字。集中一处：UI 清单约束 5 要求界面不出现「账号 / 登录 / 注册」，
// 那条由 `allTexts` 的测试守着。

import Foundation

enum SpaceWording {
    /// 出境同意那一句。原话在 `SignInSections.swift`，开头是「我的账号信息」，这一版没有账号了，改写一句；
    /// **维护者定稿后替换这里**（只有这一处）。
    static let consent = "同意把我开启的云功能数据存在位于新加坡的服务器（腾讯云）并在那里处理，用于同步与整理记忆。可随时在「我」里关掉功能或删掉云端数据。"

    static let createTitle = "开通素笺云服务"

    static let createIntro = "开通后在键盘上记的事会存到云端，每天整理成记忆卡，换手机也还在。"

    static let createButton = "开通"

    static let alreadyHave = "已经有素笺云服务了？用匹配码加入"

    static let created = "已开通"

    static let joinTitle = "加入已有的素笺云服务"

    static let joinIntro = "在已经开通的另一台设备上，打开「我 → 素笺云服务 → 添加一台设备」，把那里显示的匹配码输进来。"

    static let joinField = "匹配码"

    static let joinButton = "加入"

    static let joinHint = "8 位字母数字，中间的短横线可以不输"

    static let waiting = "等另一台设备允许…"

    static let waitingHint = "另一台设备上会弹出一条申请，允许之后就加入好了。"

    static let denied = "另一台设备没有允许这次加入，可以让它再出一张码"

    static let joined = "已加入"

    static let cancel = "取消"

    static let retry = "重新输一次"

    static let missingCode = "匹配码还差几位"

    static let expired = "这次加入过期了，请重新输一张匹配码"

    static let entryTitle = "素笺云服务"

    static let entryOpened = "已开通"

    static let entryClosed = "没开通"

    static let openedNote = "这台设备已经开通了素笺云服务。"

    static let openedMore = "设备列表、加一台设备、出匹配码在下一步做。"

    /// 开通页与输码页上会出现的全部文字，测试用来查有没有不该出现的词。
    static let allTexts: [String] = [
        consent, createTitle, createIntro, createButton, created, alreadyHave,
        joinTitle, joinIntro, joinField, joinButton, joinHint,
        waiting, waitingHint, denied, joined, cancel, retry, missingCode, expired,
        entryTitle, entryOpened, entryClosed, openedNote, openedMore,
    ]

    /// 给用户看的失败原因：这两种说法与桥给的一致，其余用桥的原文。
    static func failure(_ failure: BridgeFailure) -> String {
        switch failure.code {
        case .badCode: "匹配码不对或已经过期，请重新输一张"
        case .deviceLimit: "空间里的设备已经满了，先在旧设备上删一台再加"
        default: failure.message
        }
    }
}
