#!/usr/bin/env bash
# 把打好的 QingjianCloud.app 装到当前用户的 ~/Applications，登记登录后自动启动，并（重新）启动它。
#   install-app.sh <QingjianCloud.app 路径>
# bundle.sh --install 与输入法 pkg 的 postinstall（以登录用户身份经 sudo -u 调）都用它。
set -euo pipefail

LABEL="app.qingjian.cloud.agent"
APP_NAME="QingjianCloud.app"
source_app="${1:?用法：install-app.sh <QingjianCloud.app>}"
# 经 sudo -u 调用时 HOME 可能还是 root 的，按用户名取家目录
home="$(eval echo "~$(id -un)")"
apps_dir="$home/Applications"
agent_plist="$home/Library/LaunchAgents/$LABEL.plist"

launchctl bootout "gui/$(id -u)/$LABEL" 2>/dev/null || true
mkdir -p "$apps_dir" "$(dirname "$agent_plist")"
rm -rf "${apps_dir:?}/$APP_NAME"
ditto "$source_app" "$apps_dir/$APP_NAME"
cat > "$agent_plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>$LABEL</string>
  <key>ProgramArguments</key>
  <array><string>$apps_dir/$APP_NAME/Contents/MacOS/qingjian-cloud-mac</string></array>
  <key>RunAtLoad</key><true/>
  <!-- 崩溃自动拉起；菜单里点「退出」是正常退出，不拉起 -->
  <key>KeepAlive</key><dict><key>SuccessfulExit</key><false/></dict>
  <key>ProcessType</key><string>Interactive</string>
</dict>
</plist>
PLIST
launchctl bootstrap "gui/$(id -u)" "$agent_plist"
echo "青简 Cloud 已安装并启动：$apps_dir/$APP_NAME"
