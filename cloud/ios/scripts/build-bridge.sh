#!/usr/bin/env bash
# 把 qingjian-cloud-bridge 编成真机 + 模拟器的静态库，打成 Frameworks/QingjianBridge.xcframework，
# 再把产品数据（dict.qj、lm.qj、领域词库）与只写了服务器地址的 cloud.toml 放进 Keyboard/Data/，主 App 的 MiSans 字体经 fetch-fonts.sh 放进 App/Fonts/。
# Xcode 工程的 preBuildScript 会调它，也可手动跑；Xcode 在构建开始前就定下拷哪些资源，新出现的文件要先跑一遍它再 xcodegen / xcodebuild。
# 用法：scripts/build-bridge.sh [--debug]；数据目录默认取仓库根的 data/generated，可用 QINGJIAN_DATA 覆盖；服务器地址缺省 https://pinyin.synon.ai，可用 QJ_SERVER 覆盖。
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
# 领域词库：设置页里逐个开关，键盘按 config.toml 的 [dictionaries] domains 加载
mkdir -p "$ios_dir/Keyboard/Data/dicts"
for dict in "$data"/dicts/*.qj; do
  [[ -f "$dict" ]] || continue
  target="$ios_dir/Keyboard/Data/dicts/$(basename "$dict")"
  cmp -s "$dict" "$target" || cp "$dict" "$target"
done
# 素笺云 只在构建时定服务器地址；令牌由主 App 的账号页登录后写进 App Group 的 cloud.toml，不进安装包。
# 随包的这份只作种子：主 App 第一次打开时拷进 App Group
server="${QJ_SERVER:-https://pinyin.synon.ai}"
seed="$ios_dir/Keyboard/Data/cloud.toml"
printf 'server = "%s"\n' "$server" > "$seed.tmp"
if cmp -s "$seed.tmp" "$seed"; then
  rm "$seed.tmp"
else
  mv "$seed.tmp" "$seed"
fi
"$ios_dir/scripts/fetch-fonts.sh"
echo "QingjianBridge.xcframework（$profile）、产品数据与字体已就绪"
