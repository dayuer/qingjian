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

# 模拟器切片要 arm64 与 x86_64 两份（下面 lipo 合成通用库），少编一份新检出里 lipo 必然失败
targets=(aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios)
installed="$(rustup target list --installed)"
for target in "${targets[@]}"; do
  if ! grep -qx "$target" <<<"$installed"; then
    echo "缺 Rust 目标 $target：先跑 rustup target add ${targets[*]}" >&2
    exit 1
  fi
done
for target in "${targets[@]}"; do
  cargo build $cargo_flag --manifest-path "$cloud_dir/Cargo.toml" -p qingjian-cloud-bridge --target "$target"
done

# 先打到临时目录，全部成功才替换 Frameworks/ 里那份：中途失败就删掉旧框架的话，
# Xcode 会把「缺 xcframework」记进构建描述，文件补回来也照样报，得清 DerivedData
out="$ios_dir/Frameworks/QingjianBridge.xcframework"
staging="$cloud_dir/target/xcframework-staging/QingjianBridge.xcframework"
rm -rf "$staging"
mkdir -p "$(dirname "$staging")"
args=()
for target in aarch64-apple-ios; do
  args+=(-library "$cloud_dir/target/$target/$profile/libqingjian_cloud_bridge.a"
         -headers "$cloud_dir/crates/qingjian-cloud-bridge/include")
done
# 模拟器两个架构合成一个通用切片（lipo），不然 create-xcframework 嫌两个单架构 sim 目录等价
sim_universal="$cloud_dir/target/ios-sim-universal/$profile"
mkdir -p "$sim_universal"
lipo -create \
  "$cloud_dir/target/aarch64-apple-ios-sim/$profile/libqingjian_cloud_bridge.a" \
  "$cloud_dir/target/x86_64-apple-ios/$profile/libqingjian_cloud_bridge.a" \
  -output "$sim_universal/libqingjian_cloud_bridge.a"
args+=(-library "$sim_universal/libqingjian_cloud_bridge.a"
       -headers "$cloud_dir/crates/qingjian-cloud-bridge/include")
xcodebuild -create-xcframework "${args[@]}" -output "$staging" >/dev/null
# Swift 用 `import QingjianBridge` 要一个 module map
for headers in "$staging"/*/Headers; do
  cat > "$headers/module.modulemap" <<'MAP'
module QingjianBridge {
    header "qingjian_bridge.h"
    export *
}
MAP
done
mkdir -p "$(dirname "$out")"
rm -rf "$out"
mv "$staging" "$out"

data="${QINGJIAN_DATA:-$repo_dir/data/generated}"
# 门槛：data/generated 只有过了评测门槛（GATE_PASSED 标记，`tools/release/gate-pass.sh` 写）才拿来装机，
# 否则回退到上次发版的那份 data/generated.shipped。QINGJIAN_DATA 显式指定时不拦（那是调用者自己负责）。
if [[ -z "${QINGJIAN_DATA:-}" && ! -f "$repo_dir/data/generated/GATE_PASSED" ]]; then
  if [[ -d "$repo_dir/data/generated.shipped" ]]; then
    echo "警告：data/generated 缺 GATE_PASSED（词库/语言模型没过评测门槛）—— 回退用 data/generated.shipped 那份" >&2
    data="$repo_dir/data/generated.shipped"
  else
    echo "错误：data/generated 缺 GATE_PASSED，也没有 data/generated.shipped 可回退：先跑 tools/release/gate-pass.sh，或按 data.lock 下载发版数据" >&2
    exit 1
  fi
fi
mkdir -p "$ios_dir/Keyboard/Data"
for file in dict.qj lm.qj; do
  if [[ ! -f "$data/$file" ]]; then
    echo "缺产品数据 $data/${file}：先在仓库根目录跑 tools/release/data-fetch.sh" >&2
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
# 英文词表：键盘扩展用 .qj（mmap 零拷贝、堆驻留 0；解析 TSV 要 13MB）。没有或比 TSV 旧就现打一个。
# TSV 缺了要在构建时就失败（与技能包、字体同规矩）。
english_tsv="$data/english.tsv"
if [[ ! -f "$english_tsv" ]]; then
  echo "缺产品数据 $english_tsv：键盘没有英文候选（先跑 tools/release/data-fetch.sh）" >&2
  exit 1
fi
english_qj="$data/english.qj"
if [[ ! -f "$english_qj" || "$english_tsv" -nt "$english_qj" ]]; then
  cargo run --release -q --manifest-path "$repo_dir/Cargo.toml" -p qingjian-dict-convert -- \
    --out-dir "$data" pack english --name "青简英文词表" --license "MIT"
fi
cmp -s "$english_qj" "$ios_dir/Keyboard/Data/english.qj" || cp "$english_qj" "$ios_dir/Keyboard/Data/english.qj"
# 旧版拷过 english.tsv，不再随包：清掉免得白白进扩展
rm -f "$ios_dir/Keyboard/Data/english.tsv"
# 本地整句模型：8 位含章·通变（`dict-convert pack model --quantize` 打的 .qjm，权重 mmap，键盘里只重排不生成）。
# 缺了构建时就失败，免得装上的键盘悄悄没有神经重排。
model="${QINGJIAN_MODEL:-$repo_dir/data/models/hanzhang-tongbian/hanzhang-tongbian-q8.qjm}"
if [[ ! -f "$model" ]]; then
  echo "缺 8 位通变模型 $model：用 qingjian-dict-convert pack model --input <通变三件套> --quantize --output <它> 打一份" >&2
  exit 1
fi
mkdir -p "$ios_dir/Keyboard/Data/models"
# 旧版拷过 fp16 通变（44MB），只留 8 位这一份
find "$ios_dir/Keyboard/Data/models" -mindepth 1 ! -name hanzhang-tongbian-q8.qjm -exec rm -rf {} +
target="$ios_dir/Keyboard/Data/models/hanzhang-tongbian-q8.qjm"
cmp -s "$model" "$target" || cp "$model" "$target"
# 改写技能包：随包走，桥在运行时从 data_dir/skills 读。
mkdir -p "$ios_dir/Keyboard/Data/skills"
skills=()
for skill in "$repo_dir"/assets/skills/*.toml; do
  [ -e "$skill" ] || continue
  skills+=("$skill")
  target="$ios_dir/Keyboard/Data/skills/$(basename "$skill")"
  cmp -s "$skill" "$target" || cp "$skill" "$target"
done
# 打包前挡住「没有技能」：缺了 polish 就直接失败，别让用户装上以后才发现按钮没了
if [ ${#skills[@]} -eq 0 ] || [ ! -f "$ios_dir/Keyboard/Data/skills/polish.toml" ]; then
  echo "assets/skills/ 里没有技能包（至少要有 polish.toml）：改写会整个用不了" >&2
  exit 1
fi
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
echo "QingjianBridge.xcframework（${profile}）、产品数据与字体已就绪"
