// 「记得」首页（设计稿 02 的 2a）：顶上「M 月 · 第 N 周」与「+ 记一条」，衬线大标题「记得」，
// 7 天日历条，选中那天的事件，本周回顾，以及跟某个人走过的功勋路。
//
// 「+ 记一条」是**先快速记**：不问是谁，落成「还没归到人的」卡；当天事件里带「和谁？」与「补上」，
// 事后在「补上」里选个人归过去（D1 与 2026-10-05 用户补充）。
//
// 暂缓（数据源还没有，见 UI 清单 T5/T10/T13）：标题下的「记录中 · 恋爱」与「今天 HH:mm 整理过」、
// 「n 条新记忆待确认 · 去确认」、本周回顾的云版形态（现在按免费版：只列临近的日子与约定）、
// 功勋路的左右滑换人（先只显示最近选中的那个人）。

import SwiftUI

struct RememberView: View {
    let store: MemoryStore

    @State private var selectedDay = 0

    @State private var path: [MemoryRoute] = []

    @State private var writingNote = false

    /// 正在「补上」的那条无主素材；nil 时选人弹层收着。
    @State private var assigningMaterial: MemoryMaterial?

    var body: some View {
        NavigationStack(path: $path) {
            VStack(spacing: 0) {
                // 标题、状态行与日历条都钉住不滚：设计稿里它们在 `.scroll` 外面，只有下面几节滚
                header
                statusRow
                WeekStrip(days: days, selected: $selectedDay)
                ScrollView {
                    sections
                        .padding(.bottom, 24)
                }
                .contentMargins(.bottom, RootTab.listBottomMargin, for: .scrollContent)
                // 下拉刷新挂在滚动的那一块上：挂外层的话拉到的是固定区，刷不出来
                .refreshable { await store.reload() }
            }
            .background(Color(.systemBackground))
            .toolbar(.hidden, for: .navigationBar)
            .navigationDestination(for: MemoryRoute.self) { route in
                switch route {
                case .contact(let id): ContactDetailView(store: store, contactId: id)
                case .settings(let id): ContactSettingsView(store: store, contactId: id)
                }
            }
            // 忘掉了某个人（设置页里，或键盘那边改了名单后重读）：退掉这个人的详情与设置页
            .onChange(of: store.snapshot.contacts.map(\.id)) { _, ids in
                path.removeAll { !ids.contains($0.contactId) }
            }
            .sheet(isPresented: $writingNote) { QuickNoteSheet(store: store) }
            .sheet(item: $assigningMaterial) { material in
                AssignSheet(store: store, material: material, onPicked: { assigningMaterial = nil })
            }
        }
    }

    // MARK: - 顶部

    /// 顶部一行：大标题在左、按钮在右。三页共用 `PageHeader`，高度才对得齐。
    private var header: some View {
        PageHeader(title: "记得") { quickNoteButton }
    }

    private var quickNoteButton: some View {
        Button {
            writingNote = true
        } label: {
            HStack(spacing: 4) {
                Text("+").font(AppFont.font(size: 17))
                Text("记一条").font(AppFont.font(size: 13, weight: .medium))
            }
            .foregroundStyle(Theme.ink)
            .padding(.leading, 10)
            .padding(.trailing, 12)
            .frame(height: 28)
            .background(ColorUsage.quickNoteButton.role.color)
            .clipShape(Capsule())
        }
        .buttonStyle(.plain)
        .disabled(!store.canEdit)
        .opacity(store.canEdit ? 1 : 0.4)
        // 截图走查与 UI 测试要认这个按钮，别让 label 随内部排版变
        .accessibilityIdentifier("quickNote")
    }

    /// 状态行。设计稿这里是「● 记录中 · 恋爱」+「10 月 · 第 40 周 · 今天 HH:mm 整理过」——
    /// 记录状态与整理时间来自 2B/2C，现在只有周数那半截能显示，位置先占住。
    private var statusRow: some View {
        Text(DayEvents.monthAndWeek())
            .font(AppFont.font(size: 12.5))
            .foregroundStyle(Theme.ink3)
            .padding(.horizontal, 20)
            .padding(.bottom, 2)
            .frame(maxWidth: .infinity, alignment: .leading)
    }

    // MARK: - 分节

    private var sections: some View {
        VStack(alignment: .leading, spacing: 12) {
            sectionTitle(current.title)
            dayGroup

            sectionTitle(DayEvents.thisWeekRange(), trailing: "周日晚回顾")
            weekGroup

            if let person = milestoneContact {
                sectionTitle(
                    "功勋路 · \(person.name)", trailing: "认识 \(person.knownDays()) 天")
                milestoneCard(person)
            }
        }
        .padding(.top, 14)
    }

    private func sectionTitle(_ text: String, trailing: String? = nil) -> some View {
        HStack {
            Text(text)
            Spacer()
            if let trailing {
                Text(trailing).fontWeight(.regular)
            }
        }
        .font(AppFont.font(size: 12, weight: .medium))
        .foregroundStyle(Theme.ink3)
        .padding(.horizontal, 20)
    }

    private var dayGroup: some View {
        group {
            if current.events.isEmpty {
                Text("这天没有要记着的")
                    .font(AppFont.font(size: 15))
                    .foregroundStyle(Theme.ink3)
                    .padding(.vertical, 12)
                    .padding(.horizontal, 14)
                    .frame(maxWidth: .infinity, alignment: .leading)
            } else {
                ForEach(current.events) { event in
                    eventRow(event)
                }
            }
        }
    }

    private func eventRow(_ event: DayEvent) -> some View {
        HStack(spacing: 12) {
            avatar(event)
            VStack(alignment: .leading, spacing: 2) {
                Text(event.title)
                    .font(AppFont.font(size: 15))
                    .foregroundStyle(Theme.ink)
                Text(event.subtitle)
                    .font(AppFont.font(size: 12.5))
                    .foregroundStyle(Theme.ink3)
                    .lineLimit(2)
            }
            Spacer(minLength: 8)
            tag(event)
        }
        .padding(.vertical, 12)
        .padding(.horizontal, 14)
        .contentShape(Rectangle())
        .onTapGesture {
            // 有主的事行点进对象详情；未归人的那一行靠行尾「补上」行动
            if let contact = event.contact {
                path.append(.contact(contact.id))
            }
        }
    }

    @ViewBuilder
    private func avatar(_ event: DayEvent) -> some View {
        if let contact = event.contact {
            MemoryAvatar(name: contact.name, size: 34, font: AppFont.font(size: 34 * 0.4, weight: .medium))
        } else {
            Text(event.avatarText)
                .font(AppFont.font(size: 15, weight: .medium))
                .foregroundStyle(Theme.ink2)
                .frame(width: 34, height: 34)
                .background(Circle().fill(ColorRole.neutralSoft.color))
        }
    }

    @ViewBuilder
    private func tag(_ event: DayEvent) -> some View {
        let text = Text(event.tag).font(AppFont.font(size: 11, weight: .medium))
        if event.isAction {
            Button {
                if case .unassigned(let material) = event { assigningMaterial = material }
            } label: {
                text
                    .foregroundStyle(ColorUsage.eventActionTagInk.role.color)
                    .padding(.horizontal, 8)
                    .frame(height: 20)
                    .background(ColorUsage.eventActionTag.role.color)
                    .clipShape(Capsule())
            }
            .buttonStyle(.plain)
        } else {
            text
                .foregroundStyle(ColorUsage.eventKindTag.role.color)
                .padding(.horizontal, 8)
                .frame(height: 20)
                .overlay(Capsule().strokeBorder(Color(.separator)))
        }
    }

    private var weekGroup: some View {
        group {
            let items = store.upcoming(within: 6)
            if items.isEmpty {
                Text("这 7 天没有记下的日子和约定")
                    .font(AppFont.font(size: 15))
                    .foregroundStyle(Theme.ink3)
                    .padding(.vertical, 12)
                    .padding(.horizontal, 14)
                    .frame(maxWidth: .infinity, alignment: .leading)
            } else {
                ForEach(items) { item in
                    HStack(spacing: 12) {
                        VStack(alignment: .leading, spacing: 2) {
                            Text(item.title).font(AppFont.font(size: 15))
                            Text("\(item.contact.name) · \(item.card.kind.title)")
                                .font(AppFont.font(size: 12.5))
                                .foregroundStyle(Theme.ink3)
                        }
                        Spacer(minLength: 8)
                        Text(item.dayLabel)
                            .font(AppFont.font(size: 12))
                            .foregroundStyle(Theme.ink3)
                    }
                    .padding(.vertical, 12)
                    .padding(.horizontal, 14)
                }
            }
        }
    }

    private func milestoneCard(_ contact: MemoryContact) -> some View {
        let road = Milestones.road(contact: contact, cards: store.cards(of: contact.id))
        return VStack(spacing: 0) {
            ZStack(alignment: .top) {
                // 串起节点的横线（设计稿定位在圆点中线）
                Rectangle()
                    .fill(Color(.separator))
                    .frame(height: 1)
                    .padding(.horizontal, 28)
                    .padding(.top, 6)
                HStack(alignment: .top, spacing: 4) {
                    ForEach(road) { milestone in
                        node(milestone)
                            .frame(maxWidth: .infinity)
                    }
                }
            }
        }
        .padding(.top, 16)
        .padding(.horizontal, 14)
        .padding(.bottom, 14)
        .background(
            RoundedRectangle(cornerRadius: 14)
                .fill(Color(.secondarySystemGroupedBackground))
                .overlay(RoundedRectangle(cornerRadius: 14).strokeBorder(Color(.separator)))
        )
        .padding(.horizontal, 16)
    }

    private func node(_ milestone: Milestone) -> some View {
        VStack(spacing: 6) {
            Circle()
                .frame(width: milestone.earned ? 14 : 14)
                .foregroundStyle(
                    milestone.earned
                        ? ColorUsage.milestoneEarned.role.color
                        : ColorUsage.milestoneNext.role.color)
                .background(
                    Circle()
                        .frame(width: 20, height: 20)
                        .foregroundStyle(
                            milestone.earned ? .clear : ColorUsage.milestoneNextHalo.role.color)
                )
            Text(milestone.title)
                .font(AppFont.font(size: 12, weight: .medium))
                .foregroundStyle(
                    milestone.earned
                        ? Theme.ink
                        : ColorUsage.milestoneNext.role.color)
                .multilineTextAlignment(.center)
            Text(milestone.remaining ?? shortDate(milestone.date))
                .font(AppFont.font(size: 11))
                .foregroundStyle(Theme.ink3)
                .multilineTextAlignment(.center)
        }
    }

    private func shortDate(_ date: Date) -> String {
        let parts = MemoryDate.calendar.dateComponents([.month, .day], from: date)
        return "\(parts.month ?? 0).\(parts.day ?? 0)"
    }

    /// 设计稿的 `.group`：白底、圆角 14、一圈细线，行与行之间细分隔线。
    private func group<Content: View>(@ViewBuilder content: () -> Content) -> some View {
        VStack(spacing: 0) {
            content()
        }
        .background(
            RoundedRectangle(cornerRadius: 14)
                .fill(Color(.secondarySystemGroupedBackground))
                .overlay(RoundedRectangle(cornerRadius: 14).strokeBorder(Color(.separator)))
        )
        .padding(.horizontal, 16)
    }

    // MARK: - 数据

    private var days: [DaySlot] {
        DayEvents.week(
            upcoming: store.upcoming(within: 6), unassigned: store.unassigned)
    }

    private var current: DaySlot {
        days.indices.contains(selectedDay) ? days[selectedDay] : days[0]
    }

    /// 功勋路显示的人：最近在键盘里被选中过的那个人（state 的 used 最新），没有就按场景顺序取第一个人。
    private var milestoneContact: MemoryContact? {
        let contacts = store.snapshot.contacts
        guard !contacts.isEmpty else { return nil }
        let used = store.snapshot.state.used
        let latest = contacts
            .compactMap { contact -> (MemoryContact, Int64)? in
                guard let stamp = used[contact.id] else { return nil }
                return (contact, stamp)
            }
            .max { $0.1 < $1.1 }
        if let latest { return latest.0 }
        for scene in store.scenes {
            if let person = contacts.first(where: { $0.scene == scene.id }) { return person }
        }
        return contacts.first
    }
}
