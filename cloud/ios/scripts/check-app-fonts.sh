#!/usr/bin/env bash
# 构建后检查：主 App 包里必须有原样的 MiSansVF.ttf 与 MiSans 协议，键盘扩展里不能有字体。App target 的 postBuildScript 调它。
# 用法：scripts/check-app-fonts.sh <QingjianCloud.app 路径>
set -euo pipefail

app="${1:?用法：check-app-fonts.sh <QingjianCloud.app>}"
ttf_sha=0ddef90648998900175cfdca9a6f087a2544c182f130b0ad4f7e94a03a115e79

[[ -f "$app/MiSansVF.ttf" ]] || { echo "error: $app 里没有 MiSansVF.ttf：先跑 scripts/fetch-fonts.sh（或 build-bridge.sh），再 xcodegen generate" >&2; exit 1; }
[[ "$(shasum -a 256 "$app/MiSansVF.ttf" | awk '{print $1}')" == "$ttf_sha" ]] || { echo "error: 包里的 MiSansVF.ttf 与官方原版不一致（许可不许改字体）" >&2; exit 1; }
[[ -f "$app/MiSans-LICENSE.txt" ]] || { echo "error: $app 里没有 MiSans-LICENSE.txt（许可要求随字体附带协议）" >&2; exit 1; }
if [[ -d "$app/PlugIns" ]] && find "$app/PlugIns" -iname '*.ttf' -o -iname '*.otf' | grep -q .; then
  echo "error: 键盘扩展里不该有字体文件：" >&2
  find "$app/PlugIns" -iname '*.ttf' -o -iname '*.otf' >&2
  exit 1
fi
