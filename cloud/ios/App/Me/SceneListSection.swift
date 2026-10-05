// 「我」页的「场景」一节（设计稿 05 的 2j 那一组）：每个场景一行，点进去改名或删掉；最后一行「加一个场景」。
// 设计稿写死了恋爱/日常/工作三个、也没有增删入口，这一版按 2026-10-05 定的改成用户自建。
// 副标题写人数——设计稿那里是「记录中 · 人设温和 · 2 个应用」，那些随场景之间的行为差异一起取消了。
// 新建的弹层由「我」页挂（`.sheet` 挂在 `Section` 上不生效，得挂在 Form 那一层）。

import SwiftUI

struct SceneListSection: View {
    let store: MemoryStore

    /// 点「加一个场景」时置上；弹层与它的状态在「我」页那层。
    @Binding var adding: Bool

    var body: some View {
        Section {
            ForEach(store.scenes) { scene in
                NavigationLink {
                    SceneSettingsView(store: store, sceneId: scene.id)
                } label: {
                    VStack(alignment: .leading, spacing: 3) {
                        Text(scene.name)
                        Text(MemoryStore.Wording.peopleCount(people(in: scene.id)))
                            .font(AppFont.caption)
                            .foregroundStyle(.secondary)
                    }
                }
            }
            Button("加一个场景") { adding = true }
                .disabled(!store.canEdit)
        } header: {
            Text("场景")
        } footer: {
            Text("场景只是分组：通讯录按它分，键盘上按它切人。人数不限。")
        }
    }

    private func people(in scene: String) -> Int {
        SceneGroup.people(in: scene, from: store.snapshot.contacts).count
    }
}
