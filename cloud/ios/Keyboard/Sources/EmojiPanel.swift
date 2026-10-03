// 😀 键打开的表情面板：常用表情网格，底栏回到拼音与删除。第三方键盘调不出系统表情键盘，只能自带一份。

import SwiftUI

struct EmojiPanel: View {
    let model: KeyboardModel

    var body: some View {
        VStack(spacing: 0) {
            ScrollView {
                LazyVGrid(columns: Array(repeating: GridItem(.flexible(), spacing: 0), count: 8), spacing: 4) {
                    ForEach(EmojiCatalog.common, id: \.self) { emoji in
                        Text(emoji)
                            .font(.system(size: 30))
                            .frame(maxWidth: .infinity, minHeight: 40)
                            .onKeyboardTap { model.typeEmoji(emoji) }
                    }
                }
                .padding(.horizontal, KeyStyle.sideMargin)
            }
            HStack {
                Text("拼音")
                    .font(.system(size: 17))
                    .frame(width: 64, height: 40)
                    .onKeyboardPress { model.closePanel() }
                Spacer()
                KeyButton(key: .backspace, model: model).frame(width: 56, height: 40)
            }
            .padding(.horizontal, KeyStyle.sideMargin)
            .padding(.bottom, 4)
        }
    }
}
