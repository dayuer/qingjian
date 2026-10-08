#if DEBUG
// 只进 Debug 包：对象详情底部的「连续保存 20 次」，给真机上验 App 与键盘并发写同一个人时的锁等待与冲突合并。
// 每次都走 MemoryStore.saveCard 的正常路径（后台写、等锁、conflict 合并），一次接一次地等上一笔写完；每笔的日志见 MemorySaveLog。

import SwiftUI

struct MemoryStressSection: View {
    static let rounds = 20

    let store: MemoryStore

    let contactId: String

    @State private var done = 0

    @State private var failures = 0

    @State private var running = false

    var body: some View {
        PaperSectionTitle(text: "调试")
        PaperGroup {
            Button(running ? "连续保存中 \(done) / \(Self.rounds)" : "连续保存 \(Self.rounds) 次") { run() }
                .font(AppFont.font(size: 15))
                .foregroundStyle(ColorUsage.addCardButton.role.color)
                .disabled(running)
                .padding(.horizontal, 14)
                .padding(.vertical, 12)
                .frame(maxWidth: .infinity, alignment: .leading)
            if done > 0 && !running {
                PaperRowLine()
                Text("完成 \(done) 次，失败 \(failures) 次；每次的修订号与冲突见系统日志 memory 分类")
                    .font(AppFont.font(size: 12))
                    .foregroundStyle(Theme.ink3)
                    .padding(.horizontal, 14)
                    .padding(.vertical, 12)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
        }
    }

    private func run() {
        running = true
        done = 0
        failures = 0
        Task {
            for index in 1...Self.rounds {
                let card = MemoryCard.new(kind: .other, text: "压测 \(index)/\(Self.rounds)", when: nil, keywords: [])
                if !(await store.saveCard(card, for: contactId)) { failures += 1 }
                done = index
            }
            running = false
        }
    }
}
#endif
