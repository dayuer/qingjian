# mac 候选窗口交互自定义（2026-10-06）

## 目标

sujian 分叉对 macOS 壳的三项自定义：单行候选条 `↑`/`↓` 翻页、候选窗口只支持横排、学习语言默认关闭。
Core 不动逻辑，只改 platform 的默认值；Windows / Linux 行为不变（文档里 mac 与 Windows 的按键表自此分叉）。

## 改动一：单行 ↑/↓ 翻页

`apps/macos/src/imk/controller/display.rs` 的 `move_highlight`：

- `horizontal_grid` 开着：保持现状（单行先展开矩阵、在矩阵里换行）。
- 没开（默认单行）：改走 `turn_page(delta)`——`↑` 上一页、`↓` 下一页，照常记
  `engine.note_page_turn()`（Core 输入日志的翻页信号）。
- 实施修正：算式（`↑`/`↓` 加 Space 选第二项）与英文模式（`↑`/`↓` 移动高亮）是既有按键依赖，
  这两个模式保留 `move_highlight`，不翻页。
- 已在首页按 `↑` / 末页按 `↓`：不动、不重画。翻页后高亮落新页第一个候选（`turn_page` 现有语义）。
- 单行模式因此失去「上下键移高亮」；数字键直选不受影响。

翻页键由壳决定是既有约定（`crates/qingjian-core/src/punctuation.rs` 的注释），Core 零改动。

## 改动二：候选窗口只横排

mac 壳删干净竖排路径：

- `candidates/view/mod.rs`：`draw_vertical`、`vertical_size`、`Layout::Vertical` 分支与竖排尺寸常数。
- `candidates/bitmap/mod.rs`：`Layout::Vertical` 映射。
- `candidates/window.rs`：竖排 / 横排切换接口。
- `preferences/pages/candidates.rs` 与 `preferences/setting/mod.rs`：竖排 / 横排弹出菜单。

`[general] layout` 配置项保留（Windows / Linux 还用），mac 壳不再读它；配置里写了
`vertical` 也强制横排。

## 改动三：学习语言默认关

`crates/qingjian-platform/src/config/general.rs` 默认 `learning_language` 从 `"en"` 改 `"off"`，
连同 `config/mod.rs` 的样例配置与测试断言。共享 Config，三平台默认一致；已有配置文件里
写了语言的用户不受影响，没写的（含新装机）不再默认显示英文释义。设置页菜单保留。

## 文档同步（同一提交）

- `docs/user/getting-started/keys.md`：mac 列 `↑`/`↓` 从「移动高亮（到页边自动翻页）」改为「翻页」。
- `docs/user/settings/preferences.md`：删竖排 / 横排菜单的描述，学习语言默认值改「关」。
- 其余 `docs/user/` 里提到默认英语译文的表述随实现搜一遍同改。

## 测试与验证

- 单测：`turn_page` 的语义已有 `host/session.rs` 的 `paging_follows_the_layout` 覆盖，控制器
  glue 无 IMK 运行时测不了，靠真机验证；竖排路径在 view / bitmap / preferences 无独立测试，
  删除由编译与 clippy 兜底；`qingjian-platform` config 测试改默认值断言。
- `cargo test -p qingjian-macos -p qingjian-platform` 与全 workspace 测试过。
- 真机：`apps/macos/scripts/bundle.sh --install` 后在 TextEdit 敲拼音，验证 `↑`/`↓` 翻一整页、
  设置页无布局菜单、学习语言默认「关」。
