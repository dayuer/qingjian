// 手写记一笔、给新对象起名字共用的输入条：占提示行的位置。左边是草稿、灰色的正在组的拼音与末尾的光标，空草稿显示占位文字；
// 右边「取消」（.btn.ghost，ink-2）与「记到 {对象}」/「好了」（.btn.acc：灰绿实底、ink 字，草稿为空时置灰）。拼音不进宿主的 marked text，只画在这里。
// 工作场景不用强调色：底换成白、按钮换成中性浅底。

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
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(ColorUsage.noteCancel.role.color)
                .padding(.horizontal, 10)
                .frame(height: 28)
                .onKeyboardPress { model.cancelComposedNote() }
            Text(model.namingContact ? "好了" : "记到\(name)")
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(Theme.ink)
                .padding(.horizontal, 14)
                .frame(height: 28)
                .background(Capsule().fill(ColorUsage.noteConfirm.role(in: model.scope.scene).color))
                .opacity(composer.canSave ? 1 : 0.4)
                .padding(.trailing, 12)
                .onKeyboardPress { model.confirmComposedNote() }
        }
        .frame(height: KeyStyle.hintRowHeight)
        .background(MemoryScope.usesAccent(model.scope.scene) ? Theme.accentSoft.color : KeyStyle.keyFill)
        .overlay(alignment: .bottom) {
            Rectangle().fill(MemoryScope.usesAccent(model.scope.scene) ? Theme.hintLine.color : Color.primary.opacity(0.08))
                .frame(height: 1)
        }
    }

    private var name: String { model.currentContact?.chipName ?? "" }

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
                Text(model.namingContact ? ContactAdd.placeholder : NoteComposer.placeholder(name: model.currentContact?.chipName))
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
