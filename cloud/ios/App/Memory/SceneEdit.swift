// 「我」页对场景的一次改动：加或改名（同 id 只改名），或删掉。交给 MemoryWorker.editScene 走桥的场景接口，不走整份写。

enum SceneEdit: Sendable {
    /// 加一个场景，或给已有的改名。
    case put(MemoryScene)

    /// 删掉这个 id 的场景，里面的人挪到默认场景。
    case delete(String)
}
