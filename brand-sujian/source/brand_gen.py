"""素笺品牌定稿：生成 SVG（文字已转曲线），再用 Chrome 无头渲染 PNG / JPG。"""
import math, os, subprocess, sys
from fontTools.ttLib import TTFont
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = sys.argv[1]
FONT = TTFont(os.path.join(HERE, 'serif-sub.ttf'))
GS = FONT.getGlyphSet()
CMAP = FONT.getBestCmap()
UPM = FONT['head'].unitsPerEm
CHROME = '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome'


def oklch(l, c, h):
    a, b = c * math.cos(math.radians(h)), c * math.sin(math.radians(h))
    l_ = l + 0.3963377774 * a + 0.2158037573 * b
    m_ = l - 0.1055613458 * a - 0.0638541728 * b
    s_ = l - 0.0894841775 * a - 1.2914855480 * b
    L, M, S = l_ ** 3, m_ ** 3, s_ ** 3
    rgb = (4.0767416621 * L - 3.3077115913 * M + 0.2309699292 * S,
           -1.2684380046 * L + 2.6097574011 * M - 0.3413193965 * S,
           -0.0041960863 * L - 0.7034186147 * M + 1.7076147010 * S)
    def enc(x):
        x = min(max(x, 0), 1)
        return 12.92 * x if x <= 0.0031308 else 1.055 * x ** (1 / 2.4) - 0.055
    return '#' + ''.join(f'{round(enc(v) * 255):02x}' for v in rgb)


PAPER = oklch(0.985, 0.004, 150)
ACCENT = oklch(0.89, 0.06, 150)
FLAP = oklch(0.93, 0.012, 150)
INK_ACC = oklch(0.38, 0.05, 150)
LINE = oklch(0.88, 0.008, 150)
INK = oklch(0.18, 0, 0)
EDGE = oklch(0.89, 0, 0)


def text_path(s, size, x, y, spacing=0.0, fill='#000'):
    """把一行字转成 path；(x, y) 是基线起点，spacing 以 em 计。返回 (svg, 宽度)。"""
    scale = size / UPM
    parts, cx = [], x
    for ch in s:
        g = FONT.getGlyphOrder()[0] if ord(ch) not in CMAP else CMAP[ord(ch)]
        pen = SVGPathPen(GS)
        GS[g].draw(TransformPen(pen, (scale, 0, 0, -scale, cx, y)))
        d = pen.getCommands()
        if d:
            parts.append(d)
        cx += GS[g].width * scale + spacing * size
    width = cx - x - spacing * size
    return f'<path fill="{fill}" d="{" ".join(parts)}"/>', width


def svg(w, h, body, bg=None):
    rect = f'<rect width="{w}" height="{h}" fill="{bg}"/>' if bg else ''
    return (f'<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}">'
            f'{rect}{body}</svg>')


def icon_body(s, dark=False):
    """2b 图标，s 为边长；整张纸铺满，右上角折起。几何按 180 基准换算。"""
    k = s / 180
    if dark:
        paper, cut, flap, dot, line = oklch(0.22, 0.008, 150), oklch(0.55, 0.06, 150), oklch(0.32, 0.01, 150), ACCENT, oklch(0.40, 0.01, 150)
    else:
        paper, cut, flap, dot, line = PAPER, ACCENT, FLAP, INK_ACC, LINE
    a = 60 * k
    return (f'<defs><filter id="sh" filterUnits="userSpaceOnUse" x="0" y="0" width="{s}" height="{s}">'
            f'<feDropShadow dx="{-2*k}" dy="{2*k}" stdDeviation="{5*k}" flood-color="#002814" flood-opacity="{0.35 if dark else 0.16}"/></filter></defs>'
            f'<rect width="{s}" height="{s}" fill="{paper}"/>'
            f'<polygon points="{s-a},0 {s},0 {s},{a}" fill="{cut}"/>'
            f'<polygon points="{s-a},0 {s-a},{a} {s},{a}" fill="{flap}" filter="url(#sh)"/>'
            f'<circle cx="{46*k}" cy="{82*k}" r="{6*k}" fill="{dot}"/>'
            f'<rect x="{40*k}" y="{100*k}" width="{100*k}" height="{5*k}" rx="{2.5*k}" fill="{line}"/>'
            f'<rect x="{40*k}" y="{120*k}" width="{66*k}" height="{5*k}" rx="{2.5*k}" fill="{line}"/>')


def glyph_line(s, color, ox=0, oy=0, box=None):
    """单色线稿：圆角方纸 + 折角 + 圆点 + 两行。box 为纸所占边长（默认 0.8s，居中）。"""
    b = box or s * 0.8
    x0, y0 = ox + (s - b) / 2, oy + (s - b) / 2
    x1, y1 = x0 + b, y0 + b
    r, a, sw = b * 0.225, b * 0.3125, b * 0.04
    path = (f'M{x0+r},{y0} L{x1-a},{y0} L{x1},{y0+a} L{x1},{y1-r} Q{x1},{y1} {x1-r},{y1} '
            f'L{x0+r},{y1} Q{x0},{y1} {x0},{y1-r} L{x0},{y0+r} Q{x0},{y0} {x0+r},{y0} Z')
    u = b / 800
    return (f'<path d="{path}" fill="none" stroke="{color}" stroke-width="{sw}" stroke-linejoin="round"/>'
            f'<polygon points="{x1-a},{y0} {x1-a},{y0+a} {x1},{y0+a}" fill="{color}" stroke="{color}" stroke-width="{sw}" stroke-linejoin="round"/>'
            f'<circle cx="{x0+204*u}" cy="{y0+364*u}" r="{32*u}" fill="{color}"/>'
            f'<rect x="{x0+178*u}" y="{y0+444*u}" width="{444*u}" height="{30*u}" rx="{15*u}" fill="{color}"/>'
            f'<rect x="{x0+178*u}" y="{y0+533*u}" width="{293*u}" height="{30*u}" rx="{15*u}" fill="{color}"/>')


def rounded_icon(x, y, s, edge=True):
    """字标组合里的图标：iOS 圆角遮罩 + 一圈细边（白底上需要）。"""
    r = s * 0.2237
    clip = f'<clipPath id="ri{int(x)}{int(y)}"><rect x="{x}" y="{y}" width="{s}" height="{s}" rx="{r}"/></clipPath>'
    inner = icon_body(s).replace('id="sh"', f'id="sh{int(x)}{int(y)}"').replace('url(#sh)', f'url(#sh{int(x)}{int(y)})')
    border = f'<rect x="{x}" y="{y}" width="{s}" height="{s}" rx="{r}" fill="none" stroke="{EDGE}" stroke-width="{s/90}"/>' if edge else ''
    return f'{clip}<g clip-path="url(#ri{int(x)}{int(y)})"><g transform="translate({x},{y})">{inner}</g></g>{border}'


FILES = {}

# 图标
FILES['icon/ios-1024-light.svg'] = svg(1024, 1024, icon_body(1024))
FILES['icon/ios-1024-dark.svg'] = svg(1024, 1024, icon_body(1024, dark=True))
FILES['icon/ios-1024-tinted.svg'] = svg(1024, 1024, glyph_line(1024, '#ffffff', box=640), bg='#000000')
FILES['icon/play-store-512.svg'] = svg(512, 512, icon_body(512))

# Android 自适应：108dp 画布按 4 倍出 432，安全区直径 66dp = 264px
A = 432
sheet_x, sheet_y, sw, sh, sa = 131, 116, 170, 200, 56
fg = (f'<defs><filter id="ash" x="-20%" y="-20%" width="140%" height="140%"><feDropShadow dx="0" dy="3" stdDeviation="6" flood-color="#002814" flood-opacity=".14"/></filter></defs>'
      f'<path d="M{sheet_x+14},{sheet_y} L{sheet_x+sw-sa},{sheet_y} L{sheet_x+sw},{sheet_y+sa} L{sheet_x+sw},{sheet_y+sh-14} Q{sheet_x+sw},{sheet_y+sh} {sheet_x+sw-14},{sheet_y+sh} '
      f'L{sheet_x+14},{sheet_y+sh} Q{sheet_x},{sheet_y+sh} {sheet_x},{sheet_y+sh-14} L{sheet_x},{sheet_y+14} Q{sheet_x},{sheet_y} {sheet_x+14},{sheet_y} Z" fill="{PAPER}" filter="url(#ash)"/>'
      f'<polygon points="{sheet_x+sw-sa},{sheet_y} {sheet_x+sw-sa},{sheet_y+sa} {sheet_x+sw},{sheet_y+sa}" fill="{FLAP}" filter="url(#ash)"/>'
      f'<circle cx="{sheet_x+38}" cy="{sheet_y+80}" r="9" fill="{INK_ACC}"/>'
      f'<rect x="{sheet_x+30}" y="{sheet_y+106}" width="104" height="8" rx="4" fill="{LINE}"/>'
      f'<rect x="{sheet_x+30}" y="{sheet_y+130}" width="68" height="8" rx="4" fill="{LINE}"/>')
FILES['icon/android-foreground-432.svg'] = svg(A, A, fg)
FILES['icon/android-background-432.svg'] = svg(A, A, '', bg=ACCENT)
FILES['icon/android-monochrome-432.svg'] = svg(A, A, glyph_line(A, '#ffffff', box=220))
FILES['icon/android-preview-432.svg'] = svg(A, A, f'<clipPath id="c"><circle cx="216" cy="216" r="216"/></clipPath><g clip-path="url(#c)"><rect width="{A}" height="{A}" fill="{ACCENT}"/>{fg}</g>')

# macOS 应用图标：Big Sur 网格，1024 画布里 824 的圆角方块居中，带投影（系统不替 Mac 图标加圆角）
def mac_icon(dark=False):
    s, b, o = 1024, 824, 100
    r = b * 0.225
    inner = icon_body(b, dark).replace('id="sh"', 'id="shm"').replace('url(#sh)', 'url(#shm)')
    return (f'<defs><filter id="drop" x="-10%" y="-10%" width="120%" height="130%"><feDropShadow dx="0" dy="10" stdDeviation="12" flood-color="#000" flood-opacity=".28"/></filter>'
            f'<clipPath id="mc"><rect x="{o}" y="{o}" width="{b}" height="{b}" rx="{r}"/></clipPath></defs>'
            f'<rect x="{o}" y="{o}" width="{b}" height="{b}" rx="{r}" fill="#fff" filter="url(#drop)"/>'
            f'<g clip-path="url(#mc)"><g transform="translate({o},{o})">{inner}</g></g>'
            f'<rect x="{o}" y="{o}" width="{b}" height="{b}" rx="{r}" fill="none" stroke="{"#000" if dark else EDGE}" stroke-opacity="{.5 if dark else 1}" stroke-width="2"/>')


def menu_glyph(s):
    """输入法菜单栏模板图标：纯黑线稿、透明底，系统按深浅色反色；线条比商标粗，16pt 下看得清。"""
    b = s * 0.86
    x0, y0 = (s - b) / 2, (s - b) / 2
    x1, y1 = x0 + b, y0 + b
    r, a, sw = b * 0.2, b * 0.34, b * 0.1
    path = (f'M{x0+r},{y0} L{x1-a},{y0} L{x1},{y0+a} L{x1},{y1-r} Q{x1},{y1} {x1-r},{y1} '
            f'L{x0+r},{y1} Q{x0},{y1} {x0},{y1-r} L{x0},{y0+r} Q{x0},{y0} {x0+r},{y0} Z')
    return (f'<path d="{path}" fill="none" stroke="#000" stroke-width="{sw}" stroke-linejoin="round"/>'
            f'<polygon points="{x1-a},{y0} {x1-a},{y0+a} {x1},{y0+a}" fill="#000" stroke="#000" stroke-width="{sw}" stroke-linejoin="round"/>'
            f'<rect x="{x0+b*0.22}" y="{y0+b*0.5}" width="{b*0.56}" height="{sw}" rx="{sw/2}" fill="#000"/>'
            f'<rect x="{x0+b*0.22}" y="{y0+b*0.7}" width="{b*0.34}" height="{sw}" rx="{sw/2}" fill="#000"/>')


FILES['icon/macos-1024.svg'] = svg(1024, 1024, mac_icon())
FILES['icon/macos-menu-16.svg'] = svg(16, 16, menu_glyph(16))
FILES['icon/macos-menu-32.svg'] = svg(32, 32, menu_glyph(32))

# 字标
wm, wm_w = text_path('素笺', 124, 0, 0, spacing=0.06, fill=INK)
sub, sub_w = text_path('SUJIAN', 34, 0, 0, spacing=0.42, fill=oklch(0.45, 0, 0))
H, ic = 260, 180
x_txt = 40 + ic + 48
w_h = int(x_txt + max(wm_w, sub_w) + 48)
FILES['wordmark/lockup-horizontal.svg'] = svg(
    w_h, H, rounded_icon(40, 40, ic)
    + f'<g transform="translate({x_txt},{40+112})">{wm}</g><g transform="translate({x_txt+4},{40+112+70})">{sub}</g>', bg='#ffffff')
Wv = int(max(wm_w, sub_w) + 160)
FILES['wordmark/lockup-vertical.svg'] = svg(
    Wv, 520, rounded_icon((Wv - ic) / 2, 40, ic)
    + f'<g transform="translate({(Wv-wm_w)/2},{40+ic+170})">{wm}</g><g transform="translate({(Wv-sub_w)/2},{40+ic+240})">{sub}</g>', bg='#ffffff')
FILES['wordmark/wordmark.svg'] = svg(int(wm_w + 80), 220, f'<g transform="translate(40,170)">{wm}</g>', bg='#ffffff')

# 商标：纯黑白，无灰度、无阴影
tw, tw_w = text_path('素笺', 300, 0, 0, spacing=0.06, fill='#000000')
ts, ts_w = text_path('SUJIAN', 80, 0, 0, spacing=0.42, fill='#000000')
FILES['trademark/tm-figure.svg'] = svg(1200, 1200, glyph_line(1200, '#000000', box=900), bg='#ffffff')
Wt = int(max(tw_w, ts_w) + 300)
FILES['trademark/tm-text.svg'] = svg(Wt, 700, f'<g transform="translate({(Wt-tw_w)/2},{380})">{tw}</g><g transform="translate({(Wt-ts_w)/2},{560})">{ts}</g>', bg='#ffffff')
Wc = int(150 + 600 + 120 + max(tw_w, ts_w) + 150)
FILES['trademark/tm-combined.svg'] = svg(Wc, 900, glyph_line(600, '#000000', ox=150, oy=150, box=600)
                                         + f'<g transform="translate({150+600+120},{470})">{tw}</g><g transform="translate({150+600+128},{640})">{ts}</g>', bg='#ffffff')

for rel, content in FILES.items():
    p = os.path.join(OUT, rel)
    os.makedirs(os.path.dirname(p), exist_ok=True)
    with open(p, 'w') as f:
        f.write(content)
    w = int(content.split('width="')[1].split('"')[0]); h = int(content.split('height="')[1].split('"')[0])
    png = p[:-4] + '.png'
    html = os.path.join(HERE, 'render.html')
    with open(html, 'w') as f:
        f.write(f'<html><body style="margin:0;background:transparent">{content}</body></html>')
    subprocess.run([CHROME, '--headless=new', '--disable-gpu', '--hide-scrollbars', '--default-background-color=00000000',
                    f'--screenshot={png}', f'--window-size={w},{h}', 'file://' + html],
                   check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    if rel.startswith('trademark/'):
        Image.open(png).convert('RGB').save(png[:-4] + '.jpg', quality=92)
    print(rel, w, h)
