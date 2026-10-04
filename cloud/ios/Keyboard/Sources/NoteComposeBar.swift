// 手写记一笔、给新对象起名字共用的输入条：占提示行的位置。左边是草稿、灰色的正在组的拼音与末尾的光标，空草稿显示占位文字；
// 右边「取消」（中性色描边）与「记到 {对象}」（灰绿底，草稿为空时置灰）。拼音不进宿主的 marked text，只画在这里。

import SwiftUI

struct NoteComposeBar: View {
    let model: KeyboardModel

    let composer: NoteComposer

    var body: some View {
        HStack(spacing: 8) {
            draft
                .padding(.leading, 12)
                .frame(maxWidth: .infinity, alignment: .leading)
            Text("取消")
                .font(.system(size: 13))
                .foregroundStyle(ColorUsage.noteCancel.role.color)
                .padding(.horizontal, 10)
                .frame(height: 26)
                .overlay(RoundedRectangle(cornerRadius: 13).stroke(ColorUsage.noteCancel.role.color.opacity(0.4), lineWidth: 1))
                .onKeyboardPress { model.cancelComposedNote() }
            Text(model.namingContact ? "好了" : "记到 \(name)")
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(ColorUsage.noteConfirm.role.color)
                .padding(.horizontal, 10)
                .frame(height: 26)
                .background(Capsule().fill(Theme.accent.color))
                .opacity(composer.canSave ? 1 : 0.4)
                .padding(.trailing, 12)
                .onKeyboardPress { model.confirmComposedNote() }
        }
        .frame(height: KeyStyle.hintRowHeight)
        .background(Theme.accentSoft.color)
    }

    private var name: String { model.currentContact?.name ?? "" }

    /// 草稿太长时截掉开头，末尾与光标始终看得见。
    @ViewBuilder
    private var draft: some View {
        if let error = model.namingError {
            Text(error)
                .font(.system(size: 13))
                .foregroundStyle(.secondary)
                .lineLimit(1)
        } else if composer.text.isEmpty, model.preedit.isEmpty {
            HStack(spacing: 2) {
                cursor
                Text(model.namingContact ? ContactAdd.placeholder : NoteComposer.placeholder(name: model.currentContact?.name))
                    .font(.system(size: 14))
                    .foregroundStyle(.secondary)
                    .lineLimit(1)
            }
        } else {
            HStack(spacing: 1) {
                (Text(composer.text).foregroundStyle(Theme.ink) + Text(model.preedit).foregroundStyle(.secondary))
                    .font(.system(size: 14))
                    .lineLimit(1)
                    .truncationMode(.head)
                cursor
            }
        }
    }

    private var cursor: some View {
        Rectangle()
            .fill(Theme.ink)
            .frame(width: 2, height: 18)
    }
}
