# 素笺 品牌定稿

图标方向：整张白纸、右上角折起（Claude Design 稿 06 的 2b）。所有 SVG 里的文字都已转成曲线，不依赖字体。

## icon/

| 文件 | 用途 |
|---|---|
| ios-1024-light | App Store 与 iOS 主图标（方形满版，圆角由系统加） |
| ios-1024-dark | iOS 深色模式图标 |
| ios-1024-tinted | iOS 着色图标（黑底白线稿，系统按壁纸着色） |
| play-store-512 | Google Play 商店图标 |
| android-background-432 / android-foreground-432 | Android 自适应图标两层（108dp × 4），纸张在 66dp 安全区内 |
| android-monochrome-432 | Android 13+ 主题图标（单色） |
| android-preview-432 | 圆形遮罩下的效果预览，不用于打包 |
| macos-1024 | macOS 应用图标（Big Sur 网格：824 圆角方块居中、带投影，生成 .icns 用） |
| macos-menu.svg / .pdf | 输入法菜单栏模板图标：线框白纸，22×16pt 画布（同上游约定），纯黑透明底，系统按深浅色反色；pdf 由 `cairosvg macos-menu.svg -f pdf -o macos-menu.pdf` 导出。位图 tiff 系统不反色，别用 |
| macos-menu-16 / -32 | 同一造型的 16 / 32 像素预览，不打包 |

## wordmark/

横版组合、竖版组合、纯字标。字体为 Noto Serif SC SemiBold（SIL OFL 1.1，可用于标志）。

## trademark/

商标申请用，纯黑白、无灰度无阴影，PNG / JPG / SVG 各一份：

- tm-figure：图形商标
- tm-text：文字商标（素笺 + SUJIAN）
- tm-combined：图形与文字组合

## source/

`brand_gen.py` 生成以上全部文件（需要本机 Chrome 与 Python 的 fontTools、Pillow）：
`python3 source/brand_gen.py <输出目录>`。`serif-sub.ttf` 是只含「素笺」与 SUJIAN/Sujian 字母的 Noto Serif SC 子集。
