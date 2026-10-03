#!/usr/bin/env bash
# 生成自用的 local 分支：基线（缺省 origin/main）+ cloud/patches.txt 里列的上游 PR，每次从头重建。
# 在临时 worktree 里做，不动当前工作区；冲突的 PR 跳过并在最后列出。
#
#   cloud/scripts/build-local.sh                 # 重建 local
#   BASE=upstream/main cloud/scripts/build-local.sh
#   BRANCH=local-test cloud/scripts/build-local.sh
#   PUSH=1 cloud/scripts/build-local.sh          # 建完推到 origin（强推，local 是生成物）
set -euo pipefail

UPSTREAM_URL="https://github.com/qingjian-team/qingjian"
BRANCH="${BRANCH:-local}"
BASE="${BASE:-origin/main}"
PUSH="${PUSH:-0}"

repo="$(git rev-parse --show-toplevel)"
list="$repo/cloud/patches.txt"
[[ -f "$list" ]] || { echo "找不到 $list" >&2; exit 1; }
# macOS 自带 bash 3.2：不用 mapfile，空数组展开写成 ${a[@]+"${a[@]}"}
prs=()
while read -r pr; do prs+=("$pr"); done < <(sed 's/#.*//' "$list" | tr -s ' \t' '\n' | grep -E '^[0-9]+$')

# 找指向上游的 remote，没有就加一个 upstream
upstream="$(git remote -v | awk -v url="$UPSTREAM_URL" 'tolower($2) ~ tolower(url) && $3 == "(fetch)" {print $1; exit}')"
if [[ -z "$upstream" ]]; then
  upstream=upstream
  git remote add "$upstream" "$UPSTREAM_URL"
fi

refspecs=("+refs/heads/main:refs/remotes/$upstream/main")
for pr in ${prs[@]+"${prs[@]}"}; do
  refspecs+=("+refs/pull/$pr/head:refs/remotes/$upstream/pr/$pr")
done
# 浅克隆算不出合并基，补一段历史
depth=()
[[ "$(git rev-parse --is-shallow-repository)" == true ]] && depth=(--depth=500)
echo "拉取 $upstream main 与 ${#prs[@]} 个 PR …"
git fetch -q ${depth[@]+"${depth[@]}"} "$upstream" "${refspecs[@]}"
[[ "$BASE" == origin/* ]] && git fetch -q ${depth[@]+"${depth[@]}"} origin "+refs/heads/${BASE#origin/}:refs/remotes/$BASE"

if [[ "$(git branch --show-current)" == "$BRANCH" ]]; then
  echo "当前就在 $BRANCH 上，先切到别的分支再重建" >&2
  exit 1
fi

work="$(mktemp -d)"
trap 'git worktree remove --force "$work" >/dev/null 2>&1 || true; rm -rf "$work"' EXIT
git worktree add -q --detach "$work" "$BASE"

merged=()
skipped=()
for pr in ${prs[@]+"${prs[@]}"}; do
  ref="$upstream/pr/$pr"
  if git -C "$work" merge-base --is-ancestor "$ref" HEAD; then
    merged+=("#$pr（基线已包含）")
    continue
  fi
  if git -C "$work" merge -q --no-ff --no-edit -m "Merge upstream PR #$pr" "$ref" >/dev/null 2>&1; then
    merged+=("#$pr")
  else
    files="$(git -C "$work" diff --name-only --diff-filter=U | tr '\n' ' ')"
    git -C "$work" merge --abort
    skipped+=("#$pr 冲突：$files")
  fi
done

git branch -f "$BRANCH" "$(git -C "$work" rev-parse HEAD)"
echo
echo "$BRANCH = $BASE + ${#merged[@]} 个 PR：${merged[*]-无}"
for line in ${skipped[@]+"${skipped[@]}"}; do echo "跳过 $line"; done

if [[ "$PUSH" == 1 ]]; then
  git push -f origin "$BRANCH"
fi
