// 称呼四选一（设计稿的 .opts）：一排胶囊，选中的 ink 黑底白字，没选的细描边灰字。称呼不是代表人的元素，不用灰绿。

import SwiftUI

struct MemoryPronounPicker: View {
    @Binding var selection: MemoryPronoun

    var body: some View {
        HStack(spacing: 8) {
            ForEach(MemoryPronoun.choices, id: \.self) { choice in
                let on = choice == selection
                Button {
                    selection = choice
                } label: {
                    Text(choice.title)
                        .font(AppFont.font(size: 13))
                        .padding(.horizontal, 13)
                        .frame(height: 32)
                        .foregroundStyle(on ? Color(.systemBackground) : ColorUsage.cardNotice.role.color)
                        .background(on ? ColorUsage.editorSave.role.color : .clear, in: Capsule())
                        .overlay(Capsule().strokeBorder(Hairline.line, lineWidth: on ? 0 : 1))
                }
                .buttonStyle(.plain)
                .accessibilityAddTraits(on ? .isSelected : [])
            }
        }
    }
}
