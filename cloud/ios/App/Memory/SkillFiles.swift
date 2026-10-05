// App 侧读随包的改写技能：qj_skills 读 App 包里那份 skills/（project.yml 把 assets/skills 打进去）。
// 键盘那份在扩展包的 Data/skills 里、经会话读（Engine.rewriteSkills），两边都只有名字与说明。

import Foundation
import QingjianBridge

enum SkillFiles {
    /// App 包里技能包目录的名字（project.yml 里 assets/skills 以 folder 打进包，就在包根下）。
    static let directoryName = "skills"

    /// 读一次；没有技能包（包没打进去、或读不了）时为空。
    static func list() -> [Skill] {
        guard let directory = Bundle.main.url(forResource: directoryName, withExtension: nil) else {
            return []
        }
        let raw = directory.path.withCString { qj_skills($0) }
        return MemoryFiles.decode(MemoryFiles.take(raw)) ?? []
    }
}
