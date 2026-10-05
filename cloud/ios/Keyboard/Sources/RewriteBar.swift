// 改写进行中的候选栏（等结果 / 结果出来点一下替换原文 / 没成功两种说法），✕ 放弃；空闲时的入口在 IdleBar。
// 三种状态后面都带着技能排，结果前面写着这次用的技能名：点另一个就用它重改，并记成这个人 / 全局默认。

import SwiftUI

struct RewriteBar: View {
    let model: KeyboardModel

    var body: some View {
        HStack(spacing: 8) {
            switch model.rewrite {
            case .idle:
                EmptyView()
            case .pending:
                ProgressView().padding(.leading, 12)
                Text("改写中…").font(.system(size: 15)).foregroundStyle(.secondary)
                Spacer(minLength: 0)
                skills
                close
            case .ready(_, let skill, let result):
                Text(skill)
                    .font(.system(size: 12))
                    .foregroundStyle(Theme.ink2)
                    .lineLimit(1)
                    .padding(.leading, 12)
                Text(result)
                    .font(.system(size: 17))
                    .foregroundStyle(Theme.ink)
                    .lineLimit(1)
                    .truncationMode(.head)
                    .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .leading)
                    .onKeyboardTap { model.applyRewrite() }
                skills
                close
            case .failed, .rejected:
                Text(RewriteWording.note(for: model.rewrite) ?? "").font(.system(size: 15)).foregroundStyle(.secondary)
                    .padding(.leading, 12)
                Spacer(minLength: 0)
                skills
                close
            }
        }
    }

    /// 点另一项就用它重改（`pickRewriteSkill` 与工具栏那一排是同一个动作）。
    private var skills: some View {
        RewriteSkillRow(
            skills: model.rewriteSkills, current: model.rewriteSkillPick,
            onPick: { model.pickRewriteSkill($0) })
    }

    private var close: some View {
        Image(systemName: "xmark")
            .font(.system(size: 15, weight: .medium))
            .foregroundStyle(.secondary)
            .frame(width: 44, height: KeyStyle.candidateBarHeight)
            .onKeyboardPress { model.dismissRewrite() }
            .accessibilityLabel("放弃改写")
    }
}
