// MemoryWorker 一次保存的结果，带回主线程由 MemoryStore 更新界面与弹提示。

enum MemorySaveResult: Sendable, Equatable {
    /// 写进去了；`latest` 是写完重读的（拿新修订号），重读失败时为 nil，`merged` 表示中途遇到冲突并合并过，
    /// `brokenOnMerge` 是合并前重读时发现坏掉、已被桥备份的对象（合并结果里那人的卡片按空算）。
    case saved(written: MemorySnapshot, latest: MemorySnapshot?, merged: Bool, brokenOnMerge: [String])

    /// 桥报错（锁超时、人数上限、io 等），什么都没写。
    case failed(MemoryFailure)

    /// 冲突后重读也失败了。
    case conflictUnreadable

    /// 「忘掉」只做了一半：对象目录删掉了，名单没写进去（桥先删目录再写名单）。`latest` 是失败后重读的。
    case forgetPartial(latest: MemorySnapshot)

    /// 三轮都冲突；`latest` 是最后一次重读的，界面换成它。
    case gaveUp(latest: MemorySnapshot?)
}
