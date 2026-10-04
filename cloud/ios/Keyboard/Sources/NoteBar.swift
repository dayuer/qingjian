// 「记一笔」的确认条（设计稿 .clip）：占提示行的位置，牌子那一行保留。左边两行：小字「刚复制的」、原文一行；
// 右边「忽略」（.btn.ghost，ink-2）与「记到 {对象}」（.btn.acc：灰绿实底、ink 字）。记下后换成 .toast「记下了」（白底、ink-2、圆点），2 秒后消失。
// 工作场景的人不用强调色：「记到」换成中性浅底，toast 的圆点换成灰色。

import SwiftUI

struct NoteBar: View {
    let model: KeyboardModel

    var body: some View {
        Group {
            if let draft = model.noteDraft {
                clip(draft)
            } else {
                toast
            }
        }
        .frame(height: KeyStyle.hintRowHeight)
        .overlay(alignment: .bottom) { Rectangle().fill(Color.primary.opacity(0.08)).frame(height: 1) }
    }

    private func clip(_ draft: String) -> some View {
        HStack(spacing: 10) {
            VStack(alignment: .leading, spacing: 0) {
                Text("刚复制的")
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(Theme.ink3)
                Text(draft.replacingOccurrences(of: "\n", with: " "))
                    .font(.system(size: 13))
                    .foregroundStyle(Theme.ink2)
                    .lineLimit(1)
                    .truncationMode(.tail)
            }
            .padding(.leading, 10)
            .frame(maxWidth: .infinity, alignment: .leading)
            Text("忽略")
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(ColorUsage.noteIgnore.role.color)
                .padding(.horizontal, 10)
                .frame(height: 28)
                .onKeyboardPress { model.cancelNote() }
            Text("记到\(model.currentContact?.name ?? "")")
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(Theme.ink)
                .padding(.horizontal, 14)
                .frame(height: 28)
                .background(Capsule().fill(ColorUsage.noteConfirm.role(in: model.scope.scene).color))
                .padding(.trailing, 10)
                .onKeyboardPress { model.confirmNote() }
        }
    }

    private var toast: some View {
        HStack(spacing: 8) {
            Circle()
                .fill(MemoryScope.usesAccent(model.scope.scene) ? Theme.accent.color : Theme.ink3)
                .frame(width: 6, height: 6)
            Text("记下了")
                .font(.system(size: 12.5))
                .foregroundStyle(Theme.ink2)
            Spacer()
        }
        .padding(.leading, 12)
        .frame(maxHeight: .infinity)
        .background(KeyStyle.keyFill)
    }
}
