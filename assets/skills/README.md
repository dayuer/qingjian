# 改写技能包

一个 TOML 一个技能。`cloud/ios/scripts/build-bridge.sh` 会把它们拷进
`cloud/ios/Keyboard/Data/skills/`，桥在会话打开时从 `data_dir/skills` 读。
格式、字段约束与安全要求见 [改写技能包](../../cloud/docs/specs/rewrite-skills.md)。

**加一个技能** = 在这里加一个 `.toml` + 发新版。`id` 定了就别改：人身上按它记，改了等于换了一个技能。

提示词都是我们自己写的，没有第三方内容。
