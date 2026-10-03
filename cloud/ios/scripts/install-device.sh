#!/usr/bin/env bash
# 编 Release、自动签名、装到连着的 iPhone 并启动。
# 主 App 与键盘靠 App Group 共享设置，通配符描述文件给不了，要 Xcode 登录团队账号（Settings → Accounts）后由 xcodebuild 自动生成描述文件。
# 用法：scripts/install-device.sh [设备名或 UDID]，缺省取第一台连着的 iPhone；团队缺省 L9YRXEKYN2，可用 QJ_TEAM 覆盖。
set -euo pipefail

ios_dir="$(cd "$(dirname "$0")/.." && pwd)"
team="${QJ_TEAM:-L9YRXEKYN2}"

device="${1:-$(xcrun devicectl list devices | awk '/iPhone/ && (/available/ || / connected /) {for (i=1;i<=NF;i++) if ($i ~ /^[0-9A-F-]{36}$/) {print $i; exit}}' || true)}"
[[ -n "$device" ]] || { echo "没有连着的 iPhone" >&2; exit 1; }

cd "$ios_dir"
# 先把 xcframework、产品数据与 cloud.toml 备好：Xcode 在构建开始前就定下 Data/ 里拷哪些文件，
# 只靠工程里的 preBuildScript 生成，新出现的文件这一次不会进包
scripts/build-bridge.sh
xcodegen generate >/dev/null
xcodebuild -project QingjianCloud.xcodeproj -scheme QingjianCloud -configuration Release \
  -destination 'generic/platform=iOS' -derivedDataPath build/device \
  -allowProvisioningUpdates DEVELOPMENT_TEAM="$team" build -quiet

app=build/device/Build/Products/Release-iphoneos/QingjianCloud.app
if [[ -f cloud.local.toml && ! -f "$app/PlugIns/Keyboard.appex/Data/cloud.toml" ]]; then
  echo "包里没有 cloud.toml，装上也连不了青简 Cloud" >&2
  exit 1
fi
codesign -d --entitlements - "$app/PlugIns/Keyboard.appex" 2>/dev/null | grep -q group.app.qingjian.cloud \
  || { echo "键盘的签名里没有 App Group，设置改不到键盘上" >&2; exit 1; }

xcrun devicectl device install app --device "$device" "$app" >/dev/null
xcrun devicectl device process launch --device "$device" app.qingjian.cloud >/dev/null
echo "已装到 $device 并启动"
