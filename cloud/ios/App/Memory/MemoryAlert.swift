// 把 MemoryStore.message 弹成提示框。挂在 TabView 与每个弹出的编辑页上：编辑页盖在上面时，下面那层的提示框弹不出来，
// 而且 TabView 那层抢着弹会把编辑页连同它的提示框一起关掉（模拟器里实测），所以编辑页开着时 TabView 那层不弹（store.editorsOpen）。

import SwiftUI

struct MemoryAlert: ViewModifier {
    let store: MemoryStore

    /// 挂在弹出的编辑页上（true）还是 TabView 上（false）。
    let inEditor: Bool

    func body(content: Content) -> some View {
        content.alert(
            store.message ?? "",
            isPresented: Binding(
                get: { store.message != nil && (inEditor || store.editorsOpen == 0) },
                set: { shown in
                    if !shown { store.message = nil }
                })
        ) {
            Button("好", role: .cancel) {}
        }
        .onAppear { if inEditor { store.editorsOpen += 1 } }
        .onDisappear { if inEditor { store.editorsOpen -= 1 } }
    }
}

extension View {
    /// TabView 上的那一个。
    func memoryAlert(_ store: MemoryStore) -> some View {
        modifier(MemoryAlert(store: store, inEditor: false))
    }

    /// 弹出的编辑页上的。
    func memoryEditorAlert(_ store: MemoryStore) -> some View {
        modifier(MemoryAlert(store: store, inEditor: true))
    }
}
