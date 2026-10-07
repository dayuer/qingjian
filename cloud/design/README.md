# 素笺设计稿快照

Claude Design 项目「素笺· 移动端 UI」（原名「关系记忆 · 移动端 UI」）的导出快照，最近一次下载 2026-10-08。**以 Claude Design 里的为准**：这里只是给代码评审与 UI 审计对照用的副本，
设计稿改了就重新导出、整份覆盖 `mockups/`，并更新下表的 etag 与字节数。各屏的落地状态与开发任务见 [`../docs/plans/2026-10-05-ui-implementation.md`](../docs/plans/2026-10-05-ui-implementation.md)。

## 文件

| 文件 | 内容 | 字节 | etag |
|---|---|---|---|
| `mockups/01 键盘.dc.html` | 键盘：提示行、对象卡、牌子与快捷切人、场景与对象选择、记一笔（确认条、草稿卡、冲突）、工作场景、Android | 44536 | 1791393984737550 |
| `mockups/02 记忆卡.dc.html` | App：第 2 轮首页「记得」（日历、事件、本周、功勋路）与「通讯录」、第 2 轮「我（免费版）」；第 1 轮对象详情、改一条、对象设置与忘掉、Android | 42597 | 1791394083562680 |
| `mockups/03 每周养成.dc.html` | 每周确认：周日通知、一张一张确认、看完了、Android | 9424 | 1791091527788247 |
| `mockups/04 首次引导.dc.html` | 第 1 轮引导：介绍、开启键盘与完全访问、恢复码、第一个对象、Android | 4697 | 1791393992289454 |
| `mockups/05 记录与免费版.dc.html` | 第 2 轮：记录中 / 已暂停、免费版提示、可跳过完全访问的引导、免费与云服务、开启记录同意页、第一个对象（称呼）、首页两版、「我」、Android | 35811 | 1791393984694173 |
| `mockups/06 品牌 素笺.dc.html` | 品牌：四个 LOGO 方向（第 1 轮）与折角定稿（第 2 轮） | 25613 | 1791089890363833 |
| `mockups/theme.css` | 颜色、字体 token、所有屏共用的样式类与底部标签栏的线性图标 | 19403 | 1791393501091388 |
| `mockups/support.js` | Claude Design 的页面运行时（生成文件，不要改） | 66404 | 1791085918918273 |

字节数是写盘后逐个核对过的，与 Claude Design 里的文件大小一致。

**2026-10-08 第四次导出**：01、02、04、05 与 theme.css 改过（Claude Design 按 `writeback-prompt-2026-10-08.md` 把稿子跟上了 iOS 实现）：各屏标「已按实现更新」，新画的屏是第 3 轮（`3a`…）；App 字体改 MiSans、键盘用系统字体；「我」（05 的 2j）去掉「懒得自己写？」与数据一组，撤回同意与删云端数据都在 05 的 3c（素笺云服务页）里；02 的 2c 删掉。这次是从网页端拿的（`serve/` 接口会往 HTML 里注入两段 `data-omelette-injected` 标签，已去掉，字节数与 Claude Design 文件列表一致）。

**2026-10-05 第三次导出**：02 改过（只改了标题布局：三个页面的大标题从「`nav` 一行 + `big-t` 一行」合成一行，标题在左、按钮在右，`padding:14px 20px 6px`；2a 的周数从 nav 挪进状态行，写成「10 月 · 第 40 周 · 今天 HH:mm 整理过」）。字节数与 etag 见上表。

**2026-10-05 第二次导出**：02 与 05 两份改过，已整份覆盖。这一轮把此前「待设计确认」的 D1–D5 都答了
（首页「+ 记一条」与「记录中 · 恋爱」的位置、功勋路只用认识天数与确认卡数两种节点且不做连续打卡、
通讯录右上角「+」加人、**首字母由输入法拼音引擎算而不是 App 自己算**、场景固定三个不提供添加），
02 另新增一屏 2c「我（免费版）」。字节数已更新；上面 02、05 两行的 **etag 还要从 Claude Design 重取**（导出接口给的 etag 本地算不出来）。

**没有导出的文件**（Claude Design 的读文件接口只给文本，二进制拿不到，**需要在 Claude Design 里手动导出**后放进对应目录）：

- ~~`mockups/assets/sujian-icon-dark.png`、`sujian-icon-light.png`~~：03 的 1a、04 的 1a 引用的 App 图标，2026-10-05 第二次导出已一并补上。
  定稿图标另见 [`../brand/icon/`](../brand/icon/)。
- `mockups/uploads/draw-*.png`（7 张）：用户在设计稿上的手绘标注，`.dc.html` 不引用它们，只在 Claude Design 里看得到。

## 怎么打开

`support.js` 要用 `fetch` 读页面自身，并从 unpkg 加载 React，直接双击用 `file://` 打开时有的浏览器会拦，起一个本地静态服务最稳：

```bash
cd cloud/design/mockups
python3 -m http.server 8765 --bind 127.0.0.1
# 浏览器打开 http://127.0.0.1:8765/ ，点进任一个 .dc.html
```

`theme.css`、`support.js` 与各 `.dc.html` 用相对路径互相引用，必须放在同一个目录。需要联网（React 与 Google Fonts 走 CDN）。
标「可以试点」的几屏点得动：01 的 1c（快捷切人）、1d（切场景）、1e-2（草稿卡）、1e-3（冲突），02 的 2a（点日历换天）。

## 编号约定

引用一屏写**文件号 + 屏号**：文件号是文件名前两位，屏号是页面上灰底小标签（`dv-oid`）、也是 `dv-opt` 的 id。例如：

- `01 的 1e-2`：01 键盘里的「记一笔：拆成草稿卡」（元素 id `1e2`，标签写作 `1e-2`）；
- `02 的 2a`：02 第 2 轮的首页「记得」；
- `05 的 2f`：05 里的「开启记录」同意页。

屏号的数字前缀是轮次：`1x` 是第 1 轮，`2x` 是第 2 轮。02、06 两个文件里两轮并存，第 2 轮排在前面；同一个文件里新一轮替代旧一轮的同类屏（例如 02 的 2a 替代 1a）。
05 的第 2 轮替代了 04 的 1b 等屏。链接可以直接带锚点：`01 键盘.dc.html#1e2`。

## 颜色与字体 token

摘自 `theme.css` 的 `:root`。iOS 的对应常量在 [`cloud/ios/Shared/Theme.swift`](../ios/Shared/Theme.swift)（`enum Theme`，深色值也在那里，`Tests/ThemeTests` 用 `OKLCH.srgb` 核对换算），
哪个元素用哪一档由 [`ColorUsage`](../ios/Shared/Memory/ColorUsage.swift) / [`ColorRole`](../ios/Shared/Memory/ColorRole.swift) 统一决定，不在视图里直接挑颜色。

| token | 值（浅色） | iOS | 用途 |
|---|---|---|---|
| `--accent` | `oklch(0.89 0.06 150)` | `Theme.accent`（#C0E7C6） | 灰绿实底：圆点、选中描边、App 开关打开 |
| `--accent-ink` | `oklch(0.38 0.05 150)` | `Theme.accentInk`（#2E4A34） | 灰绿文字：牌子人名、提示行文字、首选候选 |
| `--accent-soft` | `oklch(0.965 0.025 150)` | `Theme.accentSoft`（#E8F9EB） | 灰绿浅底：提示行、牌子、今日提醒卡 |
| `--ink` | `oklch(0.18 0 0)` | `Theme.ink`（系统 `label`） | 正文 |
| `--ink-2` | `oklch(0.42 0 0)` | `Theme.ink2`（系统 `secondaryLabel`） | 次要文字、键盘工具栏按钮 |
| `--ink-3` | `oklch(0.56 0 0)` | `Theme.ink3`（#747474） | 更淡的说明：「知道了」、页脚 |
| `--paper` / `--paper-2` / `--paper-3` | `oklch(0.99 / 0.965 / 0.93 0 0)` | 系统背景色 | 页面底、灰底卡、头像灰底 |
| `--line` | `oklch(0.89 0 0)` | 系统 `separator` | 分隔线、线框按钮 |
| `--kb` / `--key` / `--key-fn` | `#d1d3d9` / `#ffffff` / `#acb1bb` | `KeyStyle`（照 iOS 26 自带键盘实测） | 键盘底、字母键、功能键 |
| `--serif` | `'Noto Serif SC', 'Songti SC', serif` | `.font(.system(..., design: .serif))` | 大标题、对象名、头像字、`.when` |
| `--sans` | `'Noto Sans SC', 'PingFang SC', system-ui` | 系统字体（苹方） | 其余文字 |

`theme.css` 里不在 `:root`、但 iOS 单独取了值的两条线：`.k-hint` 下沿 `oklch(0.91 0.035 150)` → `Theme.hintLine`，`.chip.split .cs` 右侧竖线
`oklch(0.88 0.04 150)` → `Theme.chipLine`。设计稿只有浅色，深色值是按明暗对调取的（见 `Theme.swift` 文件头）。

底部标签栏的三个线性图标 `.i-cal`（记得）、`.i-book`（通讯录）、`.i-me`（我）以 SVG data URI 写在 `theme.css` 里（24×24、1.6 线宽），
iOS 里还没有对应资源，落地时导成 template 图片放进 `Assets.xcassets`（见 UI 清单的 T3）。
