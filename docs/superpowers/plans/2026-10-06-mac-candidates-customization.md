# mac 候选窗口交互自定义 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** sujian 分叉三项自定义：单行候选 `↑`/`↓` 翻页、mac 壳只支持横排、学习语言默认关闭。

**Architecture:** Core 零逻辑改动；翻页分支改在 mac 控制器 glue（`display.rs`），竖排是纯 mac 壳删除（platform 的 `LayoutMode` 留给 Windows / Linux），默认值改在 `qingjian-platform` 共享 Config。每个任务带自己的 `docs/user/` 同步（仓库规矩：用户可感知行为与文档同一提交）。

**Tech Stack:** Rust workspace（`apps/macos` IMK 壳、`crates/qingjian-platform` Config）、objc2。

**spec:** `docs/superpowers/specs/2026-10-06-mac-candidates-customization-design.md`

**实施中发现的两个 spec 修正**（写回 spec，见 Task 3 最后一步）：
1. 算式模式（`↑`/`↓` 加 Space 选第二项，`docs/user/input/shortcuts.md:17`）与英文模式（`docs/user/input/english-mode.md:59`）的 `↑`/`↓` 是既有「移动高亮」依赖，翻页改造要避开这两个模式。
2. 「session 补单行翻页用例」不需要：`host/session.rs:247` 的 `paging_follows_the_layout` 已覆盖 `turn_page`；控制器 glue 无 IMK 运行时测不了，靠真机验证（仓库对壳层的一贯做法）。

---

### Task 1: 学习语言默认关闭（platform）

**Files:**
- Modify: `crates/qingjian-platform/src/config/general.rs:128`
- Modify: `crates/qingjian-platform/src/config/mod.rs:195`（TEMPLATE）
- Modify: `crates/qingjian-platform/src/config/mod.rs:672`（测试断言）
- Modify: `docs/user/settings/preferences.md:14`、`docs/user/getting-started/first-input.md:18`

- [ ] **Step 1: 改默认值**

`general.rs:128`（`impl Default for GeneralConfig`）：

```rust
            learning_language: "off".to_owned(),
```

- [ ] **Step 2: 同步 TEMPLATE（`template_matches_defaults` 测试要求两者相等）**

`config/mod.rs:194-195`：

```toml
# 学习语言（en 英语 / ja 日语 / es 西班牙语 / off 不显示译文）：候选旁显示哪种语言的译文，要有对应的释义表才生效
learning_language = "off"
```

- [ ] **Step 3: 改测试断言**

`config/mod.rs:672`（`general_and_shortcut_sections_parse`，其 TOML 未设 learning_language，吃默认值）：

```rust
        assert_eq!(config.general.learning_language, "off");
```

- [ ] **Step 4: 跑测试**

Run: `cargo test -p qingjian-platform`
Expected: PASS（若别处有断言默认 "en" 的测试，一并改 "off"）

- [ ] **Step 5: 同步用户文档（同一提交）**

`docs/user/settings/preferences.md:14`「通用」行：「学习语言（英语 / 日语 / 西班牙语 / 不显示译文）」改为「学习语言（缺省不显示译文；英语 / 日语 / 西班牙语）」。

`docs/user/getting-started/first-input.md:18` 改为：

```markdown
学习语言开着时（缺省关），候选旁字号较小、颜色较浅的是译词，见 [译词与生词](../learning/translation.md)。
```

- [ ] **Step 6: 提交**

```bash
git add crates/qingjian-platform/src/config/general.rs crates/qingjian-platform/src/config/mod.rs docs/user/settings/preferences.md docs/user/getting-started/first-input.md
git commit -m "feat(platform): 学习语言默认关闭

- 默认 learning_language 从 en 改 off，三平台一致；已配置用户不受影响
- 样例配置与测试断言同步

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

### Task 2: 单行候选 ↑/↓ 翻页（mac）

**Files:**
- Modify: `apps/macos/src/imk/controller/display.rs:138-152`
- Modify: `docs/user/getting-started/keys.md:45,47`

- [ ] **Step 1: 改 `move_highlight` 的分支**

`display.rs:138-152` 整段替换为（保留方法名，`command.rs:84-87` 的调用不动）：

```rust
    /// 上下键。算式（↑/↓ 加 Space 选第二项）与英文模式照旧移动高亮，越过页边自动翻页；
    /// 横排矩阵开着（`[general] horizontal_grid`）时展开矩阵换行；其余单行直接翻页
    /// （sujian 自定义，见 docs/superpowers/specs/2026-10-06-mac-candidates-customization-design.md）。
    pub(super) fn move_highlight(&self, delta: isize, client: TextClient<'_>) {
        let changed = host::with(|h| {
            if h.engine.expression_mode() || h.engine.english_mode() {
                h.session.move_highlight(delta)
            } else if h.grid_keys() {
                h.session.move_rows(delta)
            } else {
                let turned = h.session.turn_page(delta);
                if turned {
                    h.engine.note_page_turn();
                }
                turned
            }
        })
        .unwrap_or(false);
        if changed {
            self.render(client);
        }
    }
```

- [ ] **Step 2: 编译与测试**

Run: `cargo test -p qingjian-macos`
Expected: PASS（`turn_page` 语义已有 `paging_follows_the_layout` 覆盖；本步只动 glue）

- [ ] **Step 3: 同步 keys.md（同一提交）**

`docs/user/getting-started/keys.md:45`：

```markdown
| 移动高亮（到页边自动翻页） | 算式 / 英文模式 `↑` / `↓` | `↑` / `↓` |
```

`keys.md:47` mac 列加上下键（Windows 列不动）：

```markdown
| 翻页 | `↑` / `↓`、`[` `]`、`PageUp` / `PageDown`、`⇧ + Tab`（上一页） | `[` `]`、`PageUp` / `PageDown`、`Shift + Tab`（上一页） |
```

- [ ] **Step 4: 提交**

```bash
git add apps/macos/src/imk/controller/display.rs docs/user/getting-started/keys.md
git commit -m "feat(macos): 单行候选上下键翻页

- 算式与英文模式保留移高亮，矩阵模式不变，其余单行 ↑ 上一页 ↓ 下一页
- 单行模式因此没有按键移高亮，数字直选不受影响

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

### Task 3: mac 壳只支持横排

**Files:**
- Modify: `apps/macos/src/candidates/view/mod.rs`（Ivars.layout、set_layout、vertical_size、draw_vertical、Columns、两处 match）
- Modify: `apps/macos/src/candidates/bitmap/mod.rs:73,88-92`（set_frame 签名与默认 Layout）
- Modify: `apps/macos/src/candidates/window.rs:117-120`
- Modify: `apps/macos/src/host/config/mod.rs:42-49`、host 的 `layout` 字段
- Modify: `apps/macos/src/host/presenting/mod.rs:71-73`
- Modify: `apps/macos/src/preferences/pages/candidates.rs`、`preferences/setting/mod.rs`、`host/settings.rs:209-210`
- Modify: `docs/user/settings/preferences.md:15,32-33`、`docs/user/getting-started/first-input.md:17`
- Modify: `docs/superpowers/specs/2026-10-06-mac-candidates-customization-design.md`（写回两条修正）

- [ ] **Step 1: view 收敛为横排**

先读 `view/mod.rs` 的 100-125（构造）、180-200（set_layout / set_renderer 一带）再动手：

1. 删 Ivars 字段 `layout: Cell<LayoutMode>`（:36）与构造里的 `layout: Cell::new(LayoutMode::default())`（:120）。
2. 删 `pub fn set_layout`（:182-184）。
3. `set_renderer` / 位图调用处给 `bitmap.set_frame` 传布局的地方改为不传（见 Step 2 新签名），AppKit 分支删 `self.ivars().layout.get()` 的读取。
4. 尺寸 match（:205-209）替换为：

```rust
        let (body_width, body_height) = if frame.columns > 0 {
            self.matrix_size(&frame)
        } else {
            self.horizontal_size(&frame)
        };
```

5. 绘制 match（:394-398）替换为：

```rust
        if frame.columns > 0 {
            self.draw_matrix(&frame, y, bounds);
        } else {
            self.draw_horizontal(&frame, y, bounds);
        }
```

6. 删 `struct Columns`（:77 起）、`fn vertical_size`（:240 起）、`fn draw_vertical`（:454 起）及只剩竖排用的常数；清 `LayoutMode` 导入。

- [ ] **Step 2: bitmap 固定横排**

`bitmap/mod.rs`：构造处 `layout: Layout::Vertical`（:73）改 `Layout::Horizontal`；`set_frame` 删 `layout: LayoutMode` 参数，函数体里 `self.layout = match layout {…}`（:88-92）改 `self.layout = Layout::Horizontal;`；更新调用方。`qingjian_render::Layout` 导入保留（render 接口仍要布局参数）。

- [ ] **Step 3: window 与 host 去 layout**

1. `window.rs:117-120` 删 `set_layout` 方法；清 `LayoutMode` 导入（`CandidateRenderer`、`ThemeMode` 仍用）。
2. `host/config/mod.rs:42-49` 替换为：

```rust
        if self.horizontal_grid != config.general.horizontal_grid {
            self.horizontal_grid = config.general.horizontal_grid;
            self.session.collapse();
        }
```

3. 删 Host 的 `layout: LayoutMode` 字段（`host/mod.rs` 或定义处，编译器指路）及其初始化。
4. `presenting/mod.rs:71-73` 改：

```rust
    /// 横排矩阵这套按键是否生效：开关开着（`[general] horizontal_grid`）。
    pub fn grid_keys(&self) -> bool {
        self.horizontal_grid
    }
```

- [ ] **Step 4: 设置页删「排布」**

先读 `preferences/pages/candidates.rs:40-125`、`preferences/setting/mod.rs:100-120,360-380`、`host/settings.rs:200-215` 再动手：

1. 删 `CandidatesPage::layout_mode` 字段、build 处 `layout_titles` + `row_popup(…, "排布", …)`（:44-47）、同步处 `select(…, &self.layout_mode, …)`（:113-115）。
2. 删 `self.horizontal_grid.setEnabled(general.layout == LayoutMode::Horizontal)`（:118）——勾选框恒可用。
3. 删 `Setting::Layout` 枚举变体（含 :372 的清单登记），删 `settings.rs:209-210` 的写值分支。
4. 清三处 `LayoutMode` 导入。

- [ ] **Step 5: 编译、测试、clippy**

Run: `cargo clippy -p qingjian-macos --all-targets -- -D warnings && cargo test -p qingjian-macos`
Expected: 全过（编译器兜底找漏删的竖排引用）

- [ ] **Step 6: 同步文档（同一提交）**

1. `preferences.md:15` 候选窗口行删「排布（竖排 / 横排）、」。
2. `preferences.md:32` 改为「macOS 候选窗口只有横排：候选排成一行，仅高亮候选在下方单独一行显示译词，页码在行尾。Windows / Linux 可另选竖排。」
3. `preferences.md:33`「先在『候选窗口』页的排布下面勾选」改「先在『候选窗口』页勾选」；同行「不勾（缺省）按键与以前一样：`↑` / `↓` 逐个移动高亮，`←` / `→` 移动拼音光标，`Esc` 清空」改「不勾（缺省）：`↑` / `↓` 翻页，`←` / `→` 移动拼音光标，`Esc` 清空」。
4. `first-input.md:17` 末句改「macOS 只有横排；Windows / Linux 缺省竖排、可改横排。每页 9 个，可在设置中修改。」
5. spec 写回两条修正（算式 / 英文模式保留移高亮；session 测试已覆盖、无需新增）。

- [ ] **Step 7: 提交**

```bash
git add -A apps/macos docs/user docs/superpowers/specs/2026-10-06-mac-candidates-customization-design.md
git commit -m "refactor(macos)!: 候选窗口只支持横排

- 删 view/bitmap/window 的竖排路径与设置页「排布」菜单
- [general] layout 留给 Windows/Linux，mac 不再读；配置写了 vertical 也强制横排
- grid_keys 只看 horizontal_grid 开关

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

### Task 4: 收尾验证

- [ ] **Step 1: 全量检查**

Run: `cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test`
Expected: 全过（pre-push 同口径）

- [ ] **Step 2: 装机冒烟**

Run: `apps/macos/scripts/bundle.sh --install` 后注销重登（或杀掉输入法进程），TextEdit 里：
敲 `nihao` → `↓` 翻到第二页、`↑` 翻回；`v1+1` 算式里 `↑`/`↓` 仍选第二项；设置页「候选窗口」无「排布」菜单、「通用」页学习语言为「不显示译文」。

- [ ] **Step 3: 向用户报告验证结果**（osascript 能自动的部分自动做，候选窗内容读不到的部分请用户敲一遍）

---

## Self-Review

- **Spec 覆盖**：改动一 → Task 2；改动二 → Task 3；改动三 → Task 1；文档同步 → 各任务 Step「同一提交」；测试与验证 → Task 4 + 各任务测试步。spec 的两条修正（算式 / 英文模式 carve-out、session 测试说明）在 Task 3 Step 6.5 写回。✅
- **占位符**：无 TBD；Task 3 的「先读再删」步骤给了精确行号锚点与替换代码，编译器兜底。✅
- **类型一致**：`set_frame` 新签名在 Step 2 定义、Step 1.3 按新签名更新调用方；`grid_keys()` 简化后 Task 2 的调用仍成立。✅
