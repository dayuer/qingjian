// 「记一笔」的确认条：占提示行的位置（选了对象才能记，那一行一定在），牌子那一行保留。
// 左边「刚复制的」加粗加截断的原文，右边「忽略」（描边）与「记到 {对象}」（强调色底）；记下后变成一行「记下了」，2 秒后消失。

import SwiftUI

struct NoteBar: View {
    let model: KeyboardModel

    var body: some View {
        HStack(spacing: 8) {
            if let draft = model.noteDraft {
                (Text("刚复制的 ").bold() + Text(draft.replacingOccurrences(of: "\n", with: " ")))
                    .font(.system(size: 14))
                    .foregroundStyle(Theme.accentInk.color)
                    .lineLimit(1)
                    .truncationMode(.tail)
                    .padding(.leading, 12)
                    .frame(maxWidth: .infinity, alignment: .leading)
                Text("忽略")
                    .font(.system(size: 13))
                    .foregroundStyle(Theme.accentInk.color)
                    .padding(.horizontal, 10)
                    .frame(height: 26)
                    .overlay(RoundedRectangle(cornerRadius: 13).stroke(Theme.accentInk.color.opacity(0.4), lineWidth: 1))
                    .onKeyboardPress { model.cancelNote() }
                Text("记到 \(model.currentContact?.name ?? "")")
                    .font(.system(size: 13, weight: .medium))
                    .foregroundStyle(Theme.accentInk.color)
                    .padding(.horizontal, 10)
                    .frame(height: 26)
                    .background(Capsule().fill(Theme.accent.color))
                    .padding(.trailing, 12)
                    .onKeyboardPress { model.confirmNote() }
            } else {
                Text("记下了")
                    .font(.system(size: 14, weight: .medium))
                    .foregroundStyle(Theme.accentInk.color)
                    .padding(.leading, 12)
                    .frame(maxWidth: .infinity, alignment: .leading)
            }
        }
        .frame(height: KeyStyle.hintRowHeight)
        .background(Theme.accentSoft.color)
    }
}
