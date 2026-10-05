// 「记一笔」要不要为这段剪贴板出确认条（设计稿 01 的 1e）：**得有时间、地点、喜好、计划这类可记的
// 东西才出**；没有就什么都不出，也不提示「未识别」——用户复制了个验证码、地址、链接，不该被问
// 「要不要记下来」。
//
// 这只是个**本地启发式**，不是抽取（抽取在云端，见 note-materials 计划）。所以宁可宽一点：
// 误判成「可记」最多多出一次确认条，用户点「忽略」就过去了；判成「不可记」则东西再也进不来。
//
// 地点不做地名识别（Core 里没有地名表，也不该为这个引一份），靠「去 / 到 / 在 / 逛 / 玩」这类
// 动词带出来。

import Foundation

enum NoteWorth {
    /// 这段文字里有没有可记的东西。
    static func isMemorable(_ text: String) -> Bool {
        let body = text.trimmingCharacters(in: .whitespacesAndNewlines)
        guard !body.isEmpty else { return false }
        if timeWords.contains(where: body.contains) { return true }
        if feelingWords.contains(where: body.contains) { return true }
        if planWords.contains(where: body.contains) { return true }
        if placeVerbs.contains(where: body.contains) { return true }
        return timePattern.firstMatch(
            in: body, range: NSRange(body.startIndex..., in: body)) != nil
    }

    /// 时间：相对日子、星期、钟点、旬月。
    private static let timeWords = [
        "今天", "明天", "后天", "昨天", "前天", "今晚", "昨晚", "早上", "上午", "中午",
        "下午", "晚上", "凌晨", "周末", "这周", "本周", "下周", "上周", "这个月", "下个月",
        "上个月", "今年", "明年", "去年", "礼拜", "星期", "周",
    ]

    /// 喜好：吃什么、爱什么、讨厌什么。
    private static let feelingWords = [
        "喜欢", "不爱", "不爱吃", "爱吃", "爱喝", "最爱", "不喜欢", "不吃", "不喝", "讨厌",
        "偏好", "习惯", "过敏", "忌口",
    ]

    /// 计划：想做什么、约了什么。
    private static let planWords = [
        "想", "打算", "准备", "计划", "约", "要去", "一起去", "记得", "别忘了", "回头",
        "下次", "改天", "提前",
    ]

    /// 地点：靠动词带出来，不做地名识别。
    private static let placeVerbs = ["去", "到", "在", "逛", "玩", "订"]

    /// 「3 月」「5 号」「7 点」「10 天后」这类带数字的时间。
    private static let timePattern = try! NSRegularExpression(
        pattern: "[0-9０-９一二三四五六七八九十]+\\s*(月|号|日|点|天后|周后|个月后)")
}
