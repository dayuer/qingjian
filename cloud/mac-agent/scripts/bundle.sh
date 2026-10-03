#!/usr/bin/env bash
# 打包青简 Cloud 的 Mac 常驻程序：生成 build/QingjianCloud.app（ad-hoc 签名）。
#   scripts/bundle.sh              # 只打包
#   scripts/bundle.sh --install    # 装到 ~/Applications，登录后自动启动，并立即启动
#   scripts/bundle.sh --uninstall  # 停止并移除（配置与日志保留）
set -euo pipefail

LABEL="app.qingjian.cloud.agent"
APP_NAME="QingjianCloud.app"
here="$(cd "$(dirname "$0")/.." && pwd)"
cloud="$(cd "$here/.." && pwd)"
apps_dir="$HOME/Applications"
agent_plist="$HOME/Library/LaunchAgents/$LABEL.plist"

uninstall() {
  launchctl bootout "gui/$(id -u)/$LABEL" 2>/dev/null || true
  rm -f "$agent_plist"
  rm -rf "${apps_dir:?}/$APP_NAME"
  echo "已移除。配置在 ~/Library/Application Support/QingjianCloud/，日志在 ~/Library/Logs/QingjianCloud/，需要的话手动删。"
}

if [[ "${1:-}" == "--uninstall" ]]; then
  uninstall
  exit 0
fi

[[ "$(uname)" == Darwin ]] || { echo "只能在 macOS 上打包" >&2; exit 1; }

version="$(sed -n 's/^version = "\(.*\)"/\1/p' "$cloud/Cargo.toml" | head -1)"
(cd "$cloud" && cargo build --release --locked -p qingjian-cloud-mac)

app="$here/build/$APP_NAME"
rm -rf "$app"
mkdir -p "$app/Contents/MacOS"
cp "$cloud/target/release/qingjian-cloud-mac" "$app/Contents/MacOS/"
cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key><string>$LABEL</string>
  <key>CFBundleName</key><string>青简 Cloud</string>
  <key>CFBundleExecutable</key><string>qingjian-cloud-mac</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>${version%%-*}</string>
  <key>CFBundleVersion</key><string>$version</string>
  <key>LSMinimumSystemVersion</key><string>11.0</string>
  <key>LSUIElement</key><true/>
</dict>
</plist>
PLIST
codesign --force --sign - "$app"
echo "已打包 $app"

if [[ "${1:-}" == "--install" ]]; then
  launchctl bootout "gui/$(id -u)/$LABEL" 2>/dev/null || true
  mkdir -p "$apps_dir" "$(dirname "$agent_plist")"
  rm -rf "${apps_dir:?}/$APP_NAME"
  cp -R "$app" "$apps_dir/"
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
  echo "已安装并启动。第一次运行会生成配置文件：菜单栏剪贴板图标 → 打开配置文件…"
fi
