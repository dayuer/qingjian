// 首页「记得」的 7 天日历条（设计稿 02 的 2a）：每列＝星期小字（11pt ink-3）+ 日期数字（衬线 600 17pt）
// + 一个 5pt 的圆点（那天有事才画）。选中那天的底是浅灰绿（约束 1：App 里的配色照设计稿）。
// 只画与选，逻辑都在 DayEvents.week 里。

import SwiftUI

struct WeekStrip: View {
    let days: [DaySlot]

    @Binding var selected: Int

    var body: some View {
        HStack(spacing: 4) {
            ForEach(Array(days.enumerated()), id: \.element.id) { index, day in
                Button {
                    selected = index
                } label: {
                    VStack(spacing: 5) {
                        Text(day.weekday)
                            .font(AppFont.font(size: 11))
                            .foregroundStyle(Theme.ink3)
                        Text("\(day.number)")
                            .font(AppFont.font(size: 17, weight: .semibold))
                            .foregroundStyle(Theme.ink)
                        Circle()
                            .frame(width: 5, height: 5)
                            .foregroundStyle(
                                day.hasEvents ? ColorUsage.calendarEventDot.role.color : .clear)
                    }
                    .frame(maxWidth: .infinity)
                    .padding(.top, 7)
                    .padding(.bottom, 8)
                    .background(
                        index == selected ? ColorUsage.calendarSelectedDay.role.color : .clear)
                    .clipShape(RoundedRectangle(cornerRadius: 12))
                }
                .buttonStyle(.plain)
            }
        }
        .padding(.horizontal, 14)
        .padding(.top, 6)
    }
}
