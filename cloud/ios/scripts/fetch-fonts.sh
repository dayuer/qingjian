#!/usr/bin/env bash
# 取主 App 用的 MiSans 可变字体，拷到 App/Fonts/MiSansVF.ttf（不进仓库）。build-bridge.sh 会调它，也可手动跑。
# MiSans 许可（cloud/brand/third-party/MiSans-LICENSE.txt）不许单独分发字体文件，所以不进 git，构建时从小米官方下载、按 SHA256 校验，原样打进安装包。
# 缓存目录缺省 ~/Library/Caches/sujian-fonts，可用 QJ_FONT_CACHE 覆盖；把官方的 MiSans.zip 放进缓存目录就能离线构建。
# 校验不过一律报错退出，不回退到系统字体。
set -euo pipefail

ios_dir="$(cd "$(dirname "$0")/.." && pwd)"
cache="${QJ_FONT_CACHE:-$HOME/Library/Caches/sujian-fonts}"
zip_url="https://hyperos.mi.com/font-download/MiSans.zip"
zip_sha=b6aa1fc827035922612df8edf36e5609bca1c5441e25cd57572204569b7b81d9
ttf_sha=0ddef90648998900175cfdca9a6f087a2544c182f130b0ad4f7e94a03a115e79
ttf_in_zip="MiSans/可变字体/MiSansVF.ttf"
out="$ios_dir/App/Fonts/MiSansVF.ttf"

sha() { shasum -a 256 "$1" | awk '{print $1}'; }

fail() {
  echo "fetch-fonts: $1" >&2
  echo "fetch-fonts: 把官方的 MiSans.zip（$zip_url）放到 $cache/MiSans.zip 可以离线构建" >&2
  exit 1
}

mkdir -p "$cache" "$(dirname "$out")"
ttf="$cache/MiSansVF.ttf"

if [[ ! -f "$ttf" || "$(sha "$ttf")" != "$ttf_sha" ]]; then
  rm -f "$ttf"
  zip="$cache/MiSans.zip"
  if [[ ! -f "$zip" || "$(sha "$zip")" != "$zip_sha" ]]; then
    echo "fetch-fonts: 下载 MiSans（约 228MB，只下载一次）到 $cache"
    curl -fL --retry 3 -o "$zip.part" "$zip_url" || { rm -f "$zip.part"; fail "下载 $zip_url 失败"; }
    mv "$zip.part" "$zip"
  fi
  actual="$(sha "$zip")"
  [[ "$actual" == "$zip_sha" ]] || fail "$zip 的 SHA256 是 $actual，应为 $zip_sha（官方包可能换了版本，核对后再改脚本里的值）"
  unzip -p "$zip" "$ttf_in_zip" > "$ttf.part" || { rm -f "$ttf.part"; fail "$zip 里没有 $ttf_in_zip"; }
  actual="$(sha "$ttf.part")"
  [[ "$actual" == "$ttf_sha" ]] || { rm -f "$ttf.part"; fail "解出的 MiSansVF.ttf 的 SHA256 是 $actual，应为 $ttf_sha"; }
  mv "$ttf.part" "$ttf"
fi

# 只在变了时拷，免得每次构建都触发重新签名
cmp -s "$ttf" "$out" || cp "$ttf" "$out"
[[ "$(sha "$out")" == "$ttf_sha" ]] || fail "$out 校验不过"
echo "fetch-fonts: MiSansVF.ttf 已就位"
