// 一个场景的设置：改名、删掉这个场景。设计稿从没画过这一屏（它假设场景固定三个、不提供添加），
// 这一版是「场景改成用户自建」时拼的，要交 UI 审计员过。
// 删掉时里面的人挪到默认场景（列表里第一个），置顶一并取消——桥那边写盘时一样；只剩一个场景不让删。

import SwiftUI

struct SceneSettingsView: View {
    let store: MemoryStore

    let sceneId: String

    @State private var renaming = false

    @State private var confirmingDelete = false

    @Environment(\.dismiss) private var dismiss

    var body: some View {
        Form {
            if let scene = store.scenes.first(where: { $0.id == sceneId }) {
                Section {
                    LabeledContent("名字", value: scene.name)
                    Button("改名") { renaming = true }
                        .disabled(!store.canEdit)
                } header: {
                    Text("场景")
                } footer: {
                    Text("改名不影响任何数据：人和记着的事都按不变的编号归在这个场景里。")
                }
                Section {
                    LabeledContent("里面的人", value: MemoryStore.Wording.peopleCount(people))
                }
                Section {
                    Button("删掉这个场景", role: .destructive) { confirmingDelete = true }
                        .frame(maxWidth: .infinity)
                        .disabled(!store.canEdit || store.scenes.count <= 1)
                } footer: {
                    Text(
                        store.scenes.count <= 1
                            ? "至少要留一个场景。"
                            : MemoryStore.Wording.sceneHasPeople(
                                people, fallback: store.defaultScene?.name ?? ""))
                }
                // 用 .alert 不用 confirmationDialog：iOS 26 上后者是气泡，「算了」不显示（同素材的「删除」）
                .alert("删掉「\(scene.name)」？", isPresented: $confirmingDelete) {
                    Button("删掉", role: .destructive) {
                        // 删成了就退回「我」页；还留着的话（比如桥拒了）留在原页看提示
                        Task {
                            if await store.deleteScene(id: sceneId) { dismiss() }
                        }
                    }
                    Button("算了", role: .cancel) {}
                } message: {
                    Text(
                        people == 0
                            ? "这个场景里没有人。"
                            : "里面的 \(people) 个人会挪到「\(store.defaultScene?.name ?? "")」。")
                }
            } else {
                Text("这个场景没有了").foregroundStyle(.secondary)
            }
        }
        .navigationTitle("场景")
        .navigationBarTitleDisplayMode(.inline)
        .sheet(isPresented: $renaming) {
            SceneNameSheet(
                title: "改名", name: store.scenes.first { $0.id == sceneId }?.name ?? ""
            ) { name in
                Task { await store.renameScene(id: sceneId, name: name) }
            }
        }
    }

    private var people: Int {
        SceneGroup.people(in: sceneId, from: store.snapshot.contacts).count
    }
}
