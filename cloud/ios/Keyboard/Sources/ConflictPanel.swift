// 记一笔和已有记忆冲突（设计稿 01 的 1e-3）：只有新内容和已有记忆冲突时才出现这一屏，**不弹窗**。
// 旧的和新的并排摆着，由用户决定是更新还是两条都留，键盘不替人下结论。
//
// **现在只是 UI 壳**：内容来自 KeyboardModel 里的样例，真正的冲突比对要等 2C
// （Task 10 的本地抽取已被审计会话正式暂缓）。

import SwiftUI

struct ConflictPanel: View {
    let model: KeyboardModel

    var body: some View {
        VStack(spacing: 10) {
            sourceRow
            headline
            cards
            note
            Spacer(minLength: 0)
            buttons
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 10)
    }

    private var sourceRow: some View {
        HStack(alignment: .top, spacing: 8) {
            Text("原话")
                .font(.system(size: 11, weight: .medium))
                .foregroundStyle(Theme.ink3)
            Text(model.draftSource)
                .font(.system(size: 12.5))
                .foregroundStyle(Theme.ink2)
                .lineLimit(2)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    private var headline: some View {
        Text("和\(conflict.contactName)的一条记忆说法不一样")
            .font(.system(size: 13, weight: .medium))
            .foregroundStyle(Theme.ink)
            .frame(maxWidth: .infinity, alignment: .leading)
    }

    private var cards: some View {
        HStack(spacing: 8) {
            card(label: conflict.oldLabel, text: conflict.oldText, isNew: false)
            card(label: conflict.newLabel, text: conflict.newText, isNew: true)
        }
    }

    private func card(label: String, text: String, isNew: Bool) -> some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(label)
                .font(.system(size: 11))
                .foregroundStyle(isNew ? ColorUsage.conflictNewLabel.role.color : Theme.ink3)
            Text(text)
                .font(.system(size: 14))
                .foregroundStyle(isNew ? Theme.ink : Theme.ink2)
                .strikethrough(!isNew)
                .multilineTextAlignment(.leading)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
        .padding(.horizontal, 12)
        .padding(.vertical, 10)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(RoundedRectangle(cornerRadius: 10).fill(Color(UIColor.systemBackground)))
        // 新卡加一圈强调色：这是关于某个人的新记忆
        .overlay {
            if isNew {
                RoundedRectangle(cornerRadius: 10)
                    .strokeBorder(ColorUsage.conflictNewRing.role.color, lineWidth: 1.5)
            }
        }
        .onTapGesture {
            // 点哪张就选哪张（设计稿让用户自己定）
        }
    }

    private var note: some View {
        Text("「两条都留」会按时间排，改写时以新的为准")
            .font(.system(size: 11.5))
            .foregroundStyle(Theme.ink3)
            .frame(maxWidth: .infinity, alignment: .leading)
    }

    private var buttons: some View {
        HStack(spacing: 8) {
            Button("不记") { model.discardDraft() }
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(ColorUsage.noteCancel.role.color)
                .frame(height: 32)
                .padding(.horizontal, 14)
                .buttonStyle(.plain)
            Button("两条都留") { model.resolveConflict(.keepBoth) }
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(ColorUsage.conflictBoth.role.color)
                .frame(maxWidth: .infinity)
                .frame(height: 32)
                .overlay(Capsule().strokeBorder(Color(.separator)))
                .buttonStyle(.plain)
            Button("更新为新的") { model.resolveConflict(.useNew) }
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(Theme.ink)
                .frame(maxWidth: .infinity)
                .frame(height: 32)
                .background(Capsule().fill(ColorUsage.noteConfirm.role.color))
                .buttonStyle(.plain)
        }
    }

    private var conflict: NoteConflict {
        model.conflict ?? NoteConflict(
            contactName: "", oldLabel: "", oldText: "", newLabel: "", newText: "")
    }
}
