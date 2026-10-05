// 「我 → 素笺云服务」：没开通就开通；开通了先显示一句状态（设备列表、加一台设备、出匹配码是 1b Task 4）。

import SwiftUI

struct SpaceEntryView: View {
    @Bindable var store: SpaceStore

    var body: some View {
        Group {
            if store.signedIn == true {
                List {
                    Section {
                        Text(SpaceWording.openedNote)
                    } footer: {
                        Text(SpaceWording.openedMore).font(AppFont.footnote)
                    }
                }
                .navigationTitle(SpaceWording.entryTitle)
                .navigationBarTitleDisplayMode(.inline)
            } else {
                CreateSpaceView(store: store)
            }
        }
        .onAppear { store.refresh() }
    }
}
