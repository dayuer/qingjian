// 一个场景与它里面的人：App 的「补上」选人分组、键盘按场景取人。场景自己的名字在 MemoryScene 上。

struct SceneGroup: Identifiable, Equatable {
    let id: String

    let name: String

    let people: [MemoryContact]

    static func people(in scene: String, from contacts: [MemoryContact]) -> [MemoryContact] {
        contacts.filter { $0.scene == scene }
    }
}
