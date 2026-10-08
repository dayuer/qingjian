// 改一条里的关键词一节（设计稿 1c）：已有的词各占一行，右边直接露出红底白字的「删除」（照稿，不做左滑），
// 最后一行是输入框与「添加」。上限与校验仍是 MemoryLimits（最多 8 个、每个 2–8 字），满了「添加」不可点。
// 标签、分组与下面那句说明都在这儿，CardEditor 只管把 $keywords 接进来。

import SwiftUI

struct KeywordRows: View {
    @Binding var keywords: [String]

    @State private var draft = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 10) {
            PaperLabel(text: "关键词 \(keywords.count) / \(MemoryLimits.maxKeywords)")
            PaperGroup(inset: 0) {
                ForEach(Array(keywords.enumerated()), id: \.element) { index, word in
                    if index > 0 { PaperRowLine() }
                    keywordRow(word)
                }
                if !keywords.isEmpty { PaperRowLine() }
                addRow
            }
            Text("打字时出现这些词，键盘会提示这一条；不写就按这条的内容自动找。")
                .font(AppFont.font(size: 11.5))
                .lineSpacing(3)
                .foregroundStyle(Theme.ink3)
                .frame(maxWidth: .infinity, alignment: .leading)
        }
    }

    private func keywordRow(_ word: String) -> some View {
        HStack(spacing: 0) {
            Text(word)
                .font(AppFont.font(size: 15))
                .foregroundStyle(Theme.ink)
                .padding(.horizontal, 14)
                .padding(.vertical, 9)
                .frame(maxWidth: .infinity, alignment: .leading)
            Button {
                keywords.removeAll { $0 == word }
            } label: {
                Text("删除")
                    .font(AppFont.font(size: 14))
                    .foregroundStyle(.white)
                    .padding(.horizontal, 14)
                    .frame(maxHeight: .infinity)
                    .background(Theme.danger)
            }
            .buttonStyle(.plain)
            .accessibilityLabel("删掉「\(word)」")
        }
    }

    private var addRow: some View {
        HStack(spacing: 0) {
            TextField("2–8 个字", text: $draft)
                .font(AppFont.font(size: 15))
                .padding(.leading, 14)
                .frame(maxWidth: .infinity, alignment: .leading)
            Button("添加") {
                keywords.append(draft.trimmingCharacters(in: .whitespacesAndNewlines))
                draft = ""
            }
            .font(AppFont.font(size: 13, weight: .medium))
            .foregroundStyle(ColorUsage.editorConfirm.role.color)
            .disabled(!MemoryLimits.canAdd(draft, to: keywords))
            .padding(.horizontal, 14)
            .padding(.vertical, 9)
        }
    }
}
