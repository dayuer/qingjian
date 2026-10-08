// 记一笔的草稿卡（设计稿 01 的 1e-2）：点 1e 的「记到 X」后占住键位区，宿主界面不动。
// 原话留在顶部作对照、点它展开全文；每项是原生输入框，点进去就改、× 删掉；
// 拿不准的项下面画虚线、右边加问号、底下写一句为什么。只有点「记下」才写进记忆。
//
// **现在只是 UI 壳**：内容来自 KeyboardModel 里的样例，真正的内容要等 2C 的云端整理
// （Task 10 的本地抽取已被审计会话正式暂缓）。

import SwiftUI

struct DraftPanel: View {
    let model: KeyboardModel

    var body: some View {
        VStack(spacing: 10) {
            sourceRow
            fieldsCard
            Spacer(minLength: 0)
            buttons
        }
        .padding(.horizontal, 12)
        .padding(.top, 12)
        .padding(.bottom, 10)
    }

    /// 原话那一行：点它展开 / 收起全文。
    private var sourceRow: some View {
        Button {
            model.toggleDraftSource()
        } label: {
            HStack(alignment: .top, spacing: 8) {
                Text("原话")
                    .font(.system(size: 11, weight: .medium))
                    .foregroundStyle(Theme.ink3)
                Text(model.draftSource)
                    .font(.system(size: 12.5))
                    .foregroundStyle(Theme.ink2)
                    .lineLimit(model.draftSourceExpanded ? nil : 1)
                    .multilineTextAlignment(.leading)
                    .frame(maxWidth: .infinity, alignment: .leading)
                Text(model.draftSourceExpanded ? "收起" : "展开")
                    .font(.system(size: 11))
                    .foregroundStyle(Theme.ink3)
            }
        }
        .buttonStyle(.plain)
        .padding(.horizontal, 2)
    }

    private var fieldsCard: some View {
        VStack(spacing: 0) {
            ForEach(Array(model.draftFields.enumerated()), id: \.offset) { index, field in
                fieldRow(index: index, field: field)
            }
        }
        .background(RoundedRectangle(cornerRadius: 10).fill(Color(UIColor.systemBackground)))
        .clipShape(RoundedRectangle(cornerRadius: 10))
        .shadow(color: .black.opacity(0.1), radius: 0, y: 1)
    }

    private func fieldRow(index: Int, field: DraftField) -> some View {
        HStack(spacing: 10) {
            Text(field.label)
                .font(.system(size: 12))
                .foregroundStyle(Theme.ink3)
                .frame(width: 28, alignment: .leading)
            VStack(alignment: .leading, spacing: 2) {
                HStack(spacing: 6) {
                    // 拿不准的只在输入框底下画虚线（设计稿 border-bottom:1px dashed），问号在虚线外
                    TextField("", text: binding(index: index))
                        .font(.system(size: 14))
                        .foregroundStyle(Theme.ink)
                        .textFieldStyle(.plain)
                        .overlay(alignment: .bottom) {
                            if field.unsure {
                                DashedLine()
                                    .stroke(ColorUsage.draftUnsure.role.color, style: StrokeStyle(lineWidth: 1, dash: [3, 2]))
                                    .frame(height: 1)
                                    .offset(y: 3)
                            }
                        }
                    if field.unsure { unsureBadge }
                }
                if field.unsure, let why = field.why {
                    Text(why)
                        .font(.system(size: 11))
                        .foregroundStyle(ColorUsage.draftUnsure.role.color)
                }
            }
            removeButton(index: index)
        }
        .padding(.leading, 12)
        .padding(.trailing, 6)
        .padding(.vertical, 6)
        .frame(minHeight: 40)
        .overlay(alignment: .bottom) {
            Rectangle().fill(Hairline.row).frame(height: 1)
        }
    }

    /// 输入框的双向绑定。KeyboardModel 不引 SwiftUI，所以绑定在这里拼。
    private func binding(index: Int) -> Binding<String> {
        Binding(
            get: {
                model.draftFields.indices.contains(index) ? model.draftFields[index].value : ""
            },
            set: { model.updateDraftField(index: index, value: $0) })
    }

    /// 拿不准的项右边那个问号。
    private var unsureBadge: some View {
        Text("?")
            .font(.system(size: 10, weight: .semibold))
            .foregroundStyle(Theme.ink2)
            .frame(width: 16, height: 16)
            .overlay(Circle().strokeBorder(ColorUsage.draftUnsure.role.color, lineWidth: 1))
    }

    private func removeButton(index: Int) -> some View {
        Button {
            model.removeDraftField(index: index)
        } label: {
            Image(systemName: "xmark")
                .font(.system(size: 12, weight: .medium))
                .foregroundStyle(Theme.ink3)
                .frame(width: 28, height: 28)
        }
        .buttonStyle(.plain)
    }

    private var buttons: some View {
        HStack(spacing: 8) {
            Button("不记") { model.discardDraft() }
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(ColorUsage.noteCancel.role.color)
                .frame(height: 32)
                .padding(.horizontal, 14)
                .buttonStyle(.plain)
            Button("记下 \(model.draftFields.count) 条") { model.saveDraft() }
                .font(.system(size: 13, weight: .medium))
                .foregroundStyle(Theme.ink)
                .frame(maxWidth: .infinity)
                .frame(height: 32)
                .background(Capsule().fill(ColorUsage.noteConfirm.role.color))
                .buttonStyle(.plain)
        }
    }
}

/// 一条横线，给虚线描边用（Rectangle 描边会画成一圈）。
private struct DashedLine: Shape {
    func path(in rect: CGRect) -> Path {
        var path = Path()
        path.move(to: CGPoint(x: rect.minX, y: rect.midY))
        path.addLine(to: CGPoint(x: rect.maxX, y: rect.midY))
        return path
    }
}
