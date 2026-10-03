#!/usr/bin/env bash
# 把 qingjian-cloud-bridge 编成真机 + 模拟器的静态库，打成 Frameworks/QingjianBridge.xcframework，
# 再把产品数据（dict.qj、lm.qj）拷进 Keyboard/Data/。Xcode 工程的 preBuildScript 会调它，也可手动跑。
# 用法：scripts/build-bridge.sh [--debug]；数据目录默认取仓库根的 data/generated，可用 QINGJIAN_DATA 覆盖。
set -euo pipefail

ios_dir="$(cd "$(dirname "$0")/.." && pwd)"
cloud_dir="$(cd "$ios_dir/.." && pwd)"
repo_dir="$(cd "$cloud_dir/.." && pwd)"
profile=release
cargo_flag=--release
if [[ "${1:-}" == "--debug" ]]; then
  profile=debug
  cargo_flag=
fi

# Xcode 的 Run Script 环境里没有用户的 PATH
export PATH="$HOME/.cargo/bin:$PATH"
# Xcode 会注入面向 iOS 的 SDKROOT 等变量，cargo 编 build.rs（主机平台）时会被带偏
unset SDKROOT IPHONEOS_DEPLOYMENT_TARGET

targets=(aarch64-apple-ios aarch64-apple-ios-sim)
for target in "${targets[@]}"; do
  cargo build $cargo_flag --manifest-path "$cloud_dir/Cargo.toml" -p qingjian-cloud-bridge --target "$target"
done

out="$ios_dir/Frameworks/QingjianBridge.xcframework"
rm -rf "$out"
args=()
for target in "${targets[@]}"; do
  args+=(-library "$cloud_dir/target/$target/$profile/libqingjian_cloud_bridge.a"
         -headers "$cloud_dir/crates/qingjian-cloud-bridge/include")
done
xcodebuild -create-xcframework "${args[@]}" -output "$out" >/dev/null
# Swift 用 `import QingjianBridge` 要一个 module map
for headers in "$out"/*/Headers; do
  cat > "$headers/module.modulemap" <<'MAP'
module QingjianBridge {
    header "qingjian_bridge.h"
    export *
}
MAP
done

data="${QINGJIAN_DATA:-$repo_dir/data/generated}"
mkdir -p "$ios_dir/Keyboard/Data"
for file in dict.qj lm.qj; do
  if [[ ! -f "$data/$file" ]]; then
    echo "缺产品数据 $data/$file：先在仓库根目录跑 tools/release/data-fetch.sh" >&2
    exit 1
  fi
  # 只在变了时拷，免得每次构建都触发重新签名大文件
  cmp -s "$data/$file" "$ios_dir/Keyboard/Data/$file" || cp "$data/$file" "$ios_dir/Keyboard/Data/$file"
done
echo "QingjianBridge.xcframework（$profile）与产品数据已就绪"
