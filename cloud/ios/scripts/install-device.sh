#!/usr/bin/env bash
# 编 Release、用本机的通配符描述文件手动签名、装到连着的 iPhone 并启动。
# Xcode 没登录账号时自动签名用不了，才走这条路；登录了直接在 Xcode 里 Run 就行。
# 用法：scripts/install-device.sh [设备名或 UDID]，缺省取第一台连着的 iPhone。
# 签名默认用团队 L9YRXEKYN2 的开发证书，可用 QJ_TEAM / QJ_IDENTITY 覆盖。
set -euo pipefail

ios_dir="$(cd "$(dirname "$0")/.." && pwd)"
team="${QJ_TEAM:-L9YRXEKYN2}"
identity="${QJ_IDENTITY:-$(security find-identity -v -p codesigning | grep "Apple Development" \
  | while read -r _ _ name; do name="${name//\"/}"; \
      security find-certificate -c "$name" -p | openssl x509 -noout -subject | grep -q "OU=$team" && echo "$name"; done | head -1 || true)}"
[[ -n "$identity" ]] || { echo "没找到团队 $team 的 Apple Development 证书" >&2; exit 1; }

device="${1:-$(xcrun devicectl list devices | awk '/iPhone/ && /available/ {for (i=1;i<=NF;i++) if ($i ~ /^[0-9A-F-]{36}$/) {print $i; exit}}' || true)}"
[[ -n "$device" ]] || { echo "没有连着的 iPhone" >&2; exit 1; }

work="$ios_dir/build/sign"
rm -rf "$work"
mkdir -p "$work"
profile=""
for dir in ~/Library/Developer/Xcode/UserData/Provisioning\ Profiles ~/Library/MobileDevice/Provisioning\ Profiles; do
  for file in "$dir"/*.mobileprovision; do
    [[ -f "$file" ]] || continue
    security cms -D -i "$file" > "$work/profile.plist" 2>/dev/null || continue
    if [[ "$(/usr/libexec/PlistBuddy -c 'Print :Entitlements:application-identifier' "$work/profile.plist")" == "$team.*" ]]; then
      profile="$file"
      break 2
    fi
  done
done
[[ -n "$profile" ]] || { echo "没找到团队 $team 的通配符描述文件（$team.*）" >&2; exit 1; }

cd "$ios_dir"
# 先把 xcframework、产品数据与 cloud.toml 备好：Xcode 在构建开始前就定下 Data/ 里拷哪些文件，
# 只靠工程里的 preBuildScript 生成，新出现的 cloud.toml 这一次不会进包
scripts/build-bridge.sh
xcodegen generate >/dev/null
xcodebuild -project QingjianCloud.xcodeproj -scheme QingjianCloud -configuration Release \
  -destination 'generic/platform=iOS' -derivedDataPath build/device build CODE_SIGNING_ALLOWED=NO -quiet

app="$work/QingjianCloud.app"
cp -R build/device/Build/Products/Release-iphoneos/QingjianCloud.app "$app"

sign() {
  local bundle="$1" identifier="$2"
  cat > "$work/entitlements.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>application-identifier</key><string>$team.$identifier</string>
<key>com.apple.developer.team-identifier</key><string>$team</string>
<key>get-task-allow</key><true/>
</dict></plist>
PLIST
  cp "$profile" "$bundle/embedded.mobileprovision"
  codesign -f -s "$identity" --entitlements "$work/entitlements.plist" "$bundle"
}
# 先签里面的扩展再签外面的 App
sign "$app/PlugIns/Keyboard.appex" app.qingjian.cloud.keyboard
sign "$app" app.qingjian.cloud
codesign -v --deep --strict "$app"
if [[ -f cloud.local.toml && ! -f "$app/PlugIns/Keyboard.appex/Data/cloud.toml" ]]; then
  echo "包里没有 cloud.toml，装上也连不了青简 Cloud" >&2
  exit 1
fi

xcrun devicectl device install app --device "$device" "$app" >/dev/null
xcrun devicectl device process launch --device "$device" app.qingjian.cloud >/dev/null
echo "已装到 $device 并启动（签名：$identity）"
