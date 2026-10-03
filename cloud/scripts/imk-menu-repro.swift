// 复现 IMK 整理输入法菜单时的崩溃，不用装输入法：把不同结构的菜单喂给 IMK 的私有方法
// `-[_IMKServerLegacy menusDictionary_CommonWithController:]`（系统输入源菜单打开、activateServer 时都会走到它）。
//
//   swiftc -O -o /tmp/menurepro cloud/scripts/imk-menu-repro.swift
//   for v in ok after-hidden separators dynamic; do /tmp/menurepro $v >/dev/null 2>&1; echo "$v exit=$?"; done
//
// 2026-10-03 用它查清 0.1.5-local.271 / 273 的崩溃（CFRelease(NULL)，exit 133）：IMK 把菜单拆成「动作列表」
// （_actionsFromMenu:，含隐藏项）和「展开的条目」（_flattenMenu:）两份按下标对齐，**带子菜单的父项排在隐藏项之后**
// 就对不上，递归时拿到 nil 去深拷贝再 CFRelease；子菜单里有分隔线、无动作的项、运行中换子菜单都没事。
// 所以「青简 Cloud ›」必须排在「有新版本」（平时隐藏）之前，见 apps/macos/src/menubar/cloud_agent.rs。
import AppKit
import InputMethodKit

@objc class Target: NSObject {
    @objc func menuAction(_ sender: Any?) {}
}

@objc class FakeController: NSObject {
    let m: NSMenu
    init(_ m: NSMenu) { self.m = m }
    @objc func menu() -> NSMenu { m }
}

let target = Target()

func item(_ title: String, action: Bool, tag: Int = 0, hidden: Bool = false, enabled: Bool = true) -> NSMenuItem {
    let it = NSMenuItem(title: title, action: action ? #selector(Target.menuAction(_:)) : nil, keyEquivalent: "")
    if action { it.target = target; it.tag = tag }
    it.isHidden = hidden
    it.isEnabled = enabled
    return it
}

/// `-` 开头的是置灰的说明行，`---` 是分隔线。
func submenu(_ titles: [String], actions: Bool = true, tagBase: Int = 1000) -> NSMenu {
    let m = NSMenu()
    m.autoenablesItems = false
    for (i, t) in titles.enumerated() {
        if t == "---" { m.addItem(.separator()); continue }
        m.addItem(item(t, action: actions, tag: tagBase + i, enabled: !t.hasPrefix("-")))
    }
    return m
}

func cloudParent(_ titles: [String]) -> NSMenuItem {
    let p = item("青简 Cloud", action: false)
    p.submenu = submenu(titles)
    return p
}

/// 照 apps/macos/src/menubar/menu.rs 的顺序搭输入法菜单。
func build(_ variant: String) -> NSMenu {
    let menu = NSMenu()
    menu.autoenablesItems = false
    menu.addItem(item("云联想", action: true, tag: 1))
    let fuzzy = item("模糊音", action: false)
    fuzzy.submenu = submenu(["an = ang", "en = eng", "in = ing"], tagBase: 100)
    menu.addItem(fuzzy)
    switch variant {
    case "ok": // 紧挨「模糊音」（现在的做法）
        menu.addItem(cloudParent(["-已连接", "-学习数据：刚刚同步", "暂停同步", "打开配置文件…"]))
    case "separators": // 同上，子菜单带分隔线与无动作项：也不崩
        menu.addItem(cloudParent(["-已连接", "---", "-还没有剪贴板记录", "---", "暂停同步"]))
    case "dynamic": // 启动时隐藏且无子菜单，之后再挂
        menu.addItem(item("青简 Cloud", action: false, hidden: true))
    default:
        break
    }
    menu.addItem(.separator())
    menu.addItem(item("偏好设置…", action: true, tag: 2))
    menu.addItem(item("打开日志目录", action: true, tag: 3))
    menu.addItem(item("", action: true, tag: 4, hidden: true)) // 「有新版本」，平时隐藏
    if variant == "after-hidden" { // 271 / 273 的位置：崩
        menu.addItem(cloudParent(["-已连接", "暂停同步"]))
    }
    menu.addItem(.separator())
    menu.addItem(item("", action: false, hidden: true, enabled: false)) // 配置错误提示
    menu.addItem(item("青简 0.1.5", action: false, enabled: false))
    return menu
}

let variant = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "ok"
// 输入法进程里有 NSApplication，菜单序列化依赖它；没有的话连正常结构都崩
let app = NSApplication.shared
app.setActivationPolicy(.prohibited)
setvbuf(stdout, nil, _IONBF, 0)
let server = IMKServer(name: "repro_connection_\(getpid())", bundleIdentifier: "app.qingjian.repro")!
let menu = build(variant)
let controller = FakeController(menu)
let sel = NSSelectorFromString("menusDictionary_CommonWithController:")
precondition(server.responds(to: sel), "这版 IMK 没有 menusDictionary_CommonWithController:，复现程序要跟着改")
func call(_ label: String) {
    _ = server.perform(sel, with: controller)
    print("\(variant) \(label): ok")
}
call("first")
if variant == "dynamic" {
    let p = menu.items.first { $0.title == "青简 Cloud" }!
    p.submenu = submenu(["-青简 Cloud 正在启动…"]); p.isHidden = false; call("attached")
    p.submenu = submenu(["-已连接", "---", "[本机] 一段剪贴板", "---", "暂停同步"]); call("swapped")
    p.isHidden = true; p.submenu = nil; call("detached")
    p.submenu = submenu(["-已连接", "暂停同步"]); p.isHidden = false; call("reattached")
}
