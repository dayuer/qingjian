#!/usr/bin/env bash
# 发一个自用版本到自建更新服务器：打输入法 pkg（青简 Cloud 链在里面）→ 更新并签名 releases.json → 传到 VPS。
# 在 local 分支的干净检出里运行（版本号用提交数，工作区有改动就拒绝），需要 data/generated/ 的产品数据：
#
#   NOTES="回车可选候选|修剪贴板同步" cloud/scripts/publish-mac.sh
#   FORCE=1 NOTES="…" cloud/scripts/publish-mac.sh      # 线上已有同一版本号时覆盖（一般不需要）
#
# 已装的输入法每天查一次 https://<域名>/releases/releases.json，有新版就在后台下好 pkg，菜单「有新版本」点了打开安装。
# 签名私钥在本机 ~/.config/qingjian-cloud/release-signing.key（qingjian-release-sign keygen 生成），公钥在
# crates/qingjian-update/src/index/signature.rs。设计见 cloud/docs/fork-patch.md。
set -euo pipefail

HOST="${HOST:-root@43.156.128.95}"
DOMAIN="${DOMAIN:-pinyin.synon.ai}"
REMOTE_DIR="${REMOTE_DIR:-/opt/qingjian-host/releases}"
KEY_FILE="${KEY_FILE:-$HOME/.config/qingjian-cloud/release-signing.key}"
NOTES="${NOTES:-}"
# 索引里留几个版本；更早的 pkg 从服务器上删掉
KEEP="${KEEP:-5}"

say() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }

root="$(git rev-parse --show-toplevel)"
cd "$root"
[[ -z "$(git status --porcelain --untracked-files=no)" ]] || { echo "工作区有改动，先提交或还原（版本号按提交数算）" >&2; exit 1; }
[[ -f "$KEY_FILE" ]] || { echo "找不到签名私钥 $KEY_FILE" >&2; exit 1; }
[[ -f data/generated/dict.qj ]] || { echo "缺产品数据，先跑 tools/release/data-fetch.sh" >&2; exit 1; }

base="$(sed -n 's/^version = "\(.*\)"/\1/p' apps/macos/Cargo.toml | head -1)"
version="${base%%-*}-local.$(git rev-list --count HEAD)"
case "$(uname -m)" in arm64) cpu=arm64 ;; x86_64) cpu=x86_64 ;; *) echo "不认识的架构" >&2; exit 1 ;; esac
file="qingjian-$version-macos-$cpu.pkg"
staging="$root/target/publish"
rm -rf "$staging" && mkdir -p "$staging"

# 更新说明会显示在菜单与「关于」页：没写或只写了占位的省略号就不发
[[ -n "${NOTES//[[:space:]…．.|]/}" ]] || { echo "NOTES 没写：NOTES=\"这次改了什么|一行一条\" $0" >&2; exit 1; }
# 同一版本号重发会换掉线上的包（sha256 变了，已下载的要重下）：要覆盖得显式 FORCE=1
if [[ "${FORCE:-0}" != 1 ]] && curl -fsS -m 20 "https://$DOMAIN/releases/releases.json" 2>/dev/null \
  | python3 -c 'import json,sys; sys.exit(0 if any(r.get("version")==sys.argv[1] for r in json.load(sys.stdin).get("releases",[])) else 1)' "$version"; then
  echo "线上已有 ${version}：没有新提交就不用再发；确实要覆盖加 FORCE=1" >&2
  exit 1
fi

say "打包输入法 ${version}"
QINGJIAN_VERSION="$version" \
  apps/macos/scripts/bundle.sh --pkg
cp "target/pkg/$file" "$staging/$file"

# 打包过程会让 LaunchServices 把构建目录里的几份同 id 副本也登记上，系统按 id 拉输入法时可能解析到错的那份；注销掉只留正式安装的
LSREGISTER=/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister
for stray in "$root/target/macos.noindex/Sujian.app" "$root/target/macos.noindex/pkg-$cpu/root/Sujian.app"; do
  "$LSREGISTER" -u "$stray" >/dev/null 2>&1 || true
done

say "更新版本列表"
# 服务器上还没有索引（第一次发）时从空列表开始；旧索引签名不验，反正整份重签
curl -fsS -m 20 "https://$DOMAIN/releases/releases.json" -o "$staging/previous.json" 2>/dev/null || echo '{"releases":[]}' > "$staging/previous.json"
VERSION="$version" FILE="$file" CPU="$cpu" DOMAIN="$DOMAIN" NOTES="$NOTES" KEEP="$KEEP" \
  COMMIT="$(git rev-parse HEAD)" python3 - "$staging" <<'PY'
import hashlib, json, os, sys
from datetime import datetime, timezone
from pathlib import Path

staging = Path(sys.argv[1])
env = os.environ
pkg = staging / env["FILE"]
release = {
    "version": env["VERSION"],
    "date": datetime.now().strftime("%Y-%m-%d"),
    "channel": "stable",
    "notes": [n.strip() for n in env["NOTES"].split("|") if n.strip()],
    "commit": env["COMMIT"],
    "assets": [{
        "platform": "macos",
        "cpu": env["CPU"],
        "file": env["FILE"],
        "url": f"https://{env['DOMAIN']}/releases/{env['FILE']}",
        "size": pkg.stat().st_size,
        "sha256": hashlib.sha256(pkg.read_bytes()).hexdigest(),
    }],
}
previous = json.loads((staging / "previous.json").read_text())
releases = [release] + [r for r in previous.get("releases", []) if r.get("version") != env["VERSION"]]
releases = releases[: int(env["KEEP"])]
index = {
    "schema_version": 1,
    "generated": datetime.now(timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    "latest": env["VERSION"],
    "releases": releases,
}
(staging / "releases.json").write_text(json.dumps(index, ensure_ascii=False, indent=2) + "\n")
keep = {a["file"] for r in releases for a in r.get("assets", [])}
(staging / "keep.txt").write_text("\n".join(sorted(keep)) + "\n")
print(f"{len(releases)} 个版本，最新 {env['VERSION']}")
PY

say "签名"
QINGJIAN_INDEX_SIGNING_KEY="$(cat "$KEY_FILE")" cargo run -q --release -p qingjian-release-sign -- sign "$staging/releases.json"
cargo run -q --release -p qingjian-release-sign -- verify "$staging/releases.json"

say "上传到 $HOST:$REMOTE_DIR"
# 先传 pkg，再传索引：客户端看到新索引时 pkg 一定已经在了。索引与签名先传成 .new 再一起改名
ssh "$HOST" "mkdir -p '$REMOTE_DIR'"
scp -q "$staging/$file" "$HOST:$REMOTE_DIR/$file"
scp -q "$staging/releases.json" "$HOST:$REMOTE_DIR/releases.json.new"
scp -q "$staging/releases.json.sig" "$HOST:$REMOTE_DIR/releases.json.sig.new"
scp -q "$staging/keep.txt" "$HOST:$REMOTE_DIR/.keep.txt"
ssh "$HOST" "cd '$REMOTE_DIR' && mv releases.json.new releases.json && mv releases.json.sig.new releases.json.sig \
  && for f in *.pkg; do grep -qxF \"\$f\" .keep.txt || rm -f -- \"\$f\"; done && rm -f .keep.txt && ls -l"

say "检查线上"
# 服务器的 nginx 开着 open_file_cache：换了文件后旧的还会再给约一分钟，等它换过来
online=""
for _ in $(seq 1 18); do
  online="$(curl -fsS -m 20 "https://$DOMAIN/releases/releases.json" | python3 -c 'import json,sys; print(json.load(sys.stdin)["latest"])' 2>/dev/null || true)"
  [[ "$online" == "$version" ]] && break
  sleep 5
done
echo "线上最新: ${online:-读不到}"
[[ "$online" == "$version" ]] || echo "注意：90 秒后线上还不是 ${version}，去服务器看看 $REMOTE_DIR" >&2
curl -fsSI -m 20 "https://$DOMAIN/releases/$file" | head -1
echo "已发布 ${version}。已装的青简一天内会查到；想马上试：偏好设置 → 关于 → 立即检查。"
