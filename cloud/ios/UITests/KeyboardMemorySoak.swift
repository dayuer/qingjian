// 键盘内存浸泡：照用户打字的样子在「试一试」框里连打 50 句（拼音取自 data/eval/sentences.tsv，项目文档句子），
// 每句停一下等本地整句模型重排，再按空格上屏首选。外面的脚本（tools/neural-quant/sim-soak.sh）在跑的同时
// 每 0.5 秒量一次键盘扩展进程的 footprint。坐标同 CandidateBarScroll（390×844）。
import XCTest

final class KeyboardMemorySoak: XCTestCase {
    private let row0: CGFloat = 572.33
    private let row1: CGFloat = 628.33
    private let row2: CGFloat = 684.33

    /// 空格键中心（pt）：量自 390×844 截图。
    private let space = CGPoint(x: 195, y: 740)

    private let sentences = [
        "zhegemululide",
        "shigeiyonghukande",
        "buxuanran",
        "kaishishiyong",
        "diyicishuru",
        "anjianyukuaijiejian",
        "yingwenmoshi",
        "pinxiejiucuo",
        "mohuyinyushurufangan",
        "kuaijieshuru",
        "wenjianjialide",
        "geifenzubiaotiyushunxu",
        "wenjianjialideqita",
        "shifenzuxiadeyemian",
        "yemianbiaoti",
        "yeshicelanlidemingzi",
        "zizuoyou",
        "lianzifu",
        "zhongwenzhixiezai",
        "jiushilujing",
        "xiangduilujingyinyong",
        "jietuyongqiansewaiguan",
        "xitongqueshengzihao",
        "zhengwendiyixingbuzaixie",
        "biaotilaizi",
        "zhengwencong",
        "shuomingshuti",
        "chenshuju",
        "zheleikouyuzhuciyufuci",
        "butiaokan",
        "tiaojianyong",
        "kexingyong",
        "biyaoyong",
        "shuyuguding",
        "houxuanchuangkou",
        "pinyinxing",
        "zuhejian",
        "mianxiangputongyonghu",
        "buchuxianshixianci",
        "quanxianzhelei",
        "yaozhilushishuo",
        "pianhaoshezhi",
        "bushuowenjianming",
        "shujuwenjianweizhizhizai",
        "yichuliechu",
        "anjiananjianmaoxie",
        "xiushijianyongfuhao",
        "yongdanci",
        "caidanyuyemingyong",
        "xitongshezhi",
    ]

    private func tap(_ app: XCUIApplication, _ x: CGFloat, _ y: CGFloat) {
        app.coordinate(withNormalizedOffset: CGVector(dx: 0, dy: 0))
            .withOffset(CGVector(dx: x, dy: y))
            .tap()
    }

    private func typeKey(_ app: XCUIApplication, _ key: Character) {
        let rows = ["qwertyuiop", "asdfghjkl", "zxcvbnm"]
        for (row, letters) in rows.enumerated() where letters.contains(key) {
            tap(app, KeyGeometry.centerX(key, row: row) ?? 0, [row0, row1, row2][row])
            return
        }
    }

    func testTypeFiftySentences() throws {
        let app = XCUIApplication()
        app.launch()
        Thread.sleep(forTimeInterval: 3)
        for label in ["开始", "先跳过", "先用免费版", "取消"] {
            let button = app.buttons[label]
            if button.waitForExistence(timeout: 3) {
                button.tap()
                Thread.sleep(forTimeInterval: 1)
            }
        }
        // 首页「+ 记一条」打开的编辑框：系统给的就是我们的键盘
        let compose = app.buttons["+ 记一条"]
        if compose.waitForExistence(timeout: 5) { compose.tap() } else { tap(app, 332, 81) }
        Thread.sleep(forTimeInterval: 2)
        tap(app, 195, 250)
        // 等键盘起来、模型在后台加载完
        Thread.sleep(forTimeInterval: 5)
        for sentence in sentences {
            for key in sentence { typeKey(app, key) }
            // 停键等重排（去抖 80ms + 打分），再上屏首选
            Thread.sleep(forTimeInterval: 0.6)
            tap(app, space.x, space.y)
        }
        let attachment = XCTAttachment(screenshot: XCUIScreen.main.screenshot())
        attachment.name = "soak-done"
        attachment.lifetime = .keepAlways
        add(attachment)
    }
}
