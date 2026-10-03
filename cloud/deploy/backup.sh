#!/usr/bin/env bash
# 备份数据库：用 SQLite 的在线备份拷一份一致的快照，服务不用停。放进 crontab 每天跑一次：
#   0 4 * * * /path/to/cloud/deploy/backup.sh /path/to/backups
set -euo pipefail
dest="${1:?用法：backup.sh <备份目录>}"
here="$(cd "$(dirname "$0")" && pwd)"
mkdir -p "$dest"
stamp="$(date +%Y%m%d-%H%M%S)"
compose=(docker compose -f "$here/docker-compose.yml")
"${compose[@]}" exec -T cloud qingjian-cloud backup "/data/backup-$stamp.sqlite3"
"${compose[@]}" cp "cloud:/data/backup-$stamp.sqlite3" "$dest/"
"${compose[@]}" exec -T cloud rm "/data/backup-$stamp.sqlite3"
# 只留最近 14 份
ls -1t "$dest"/backup-*.sqlite3 | tail -n +15 | xargs -r rm --
echo "已备份到 $dest/backup-$stamp.sqlite3"
