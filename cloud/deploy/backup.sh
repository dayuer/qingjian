#!/usr/bin/env bash
# 备份数据库：用 SQLite 的在线备份拷一份一致的快照，服务不用停。放进 root 的 crontab 每天跑一次：
#   0 4 * * * /path/to/cloud/deploy/backup.sh /path/to/backups
# 镜像是 distroless，没有 rm，快照从宿主机的卷目录直接移走，所以要 root。
set -euo pipefail
dest="${1:?用法：backup.sh <备份目录>}"
here="$(cd "$(dirname "$0")" && pwd)"
mkdir -p "$dest" && chmod 700 "$dest"
stamp="$(date +%Y%m%d-%H%M%S)"
compose=(docker compose -f "$here/docker-compose.yml")
vol="$(docker volume inspect deploy_cloud_data --format '{{.Mountpoint}}')"
"${compose[@]}" exec -T cloud qingjian-cloud backup "/data/backup-$stamp.sqlite3" </dev/null
mv "$vol/backup-$stamp.sqlite3" "$dest/"
chown root:root "$dest/backup-$stamp.sqlite3" && chmod 600 "$dest/backup-$stamp.sqlite3"
# 只留最近 14 份
ls -1t "$dest"/backup-*.sqlite3 | tail -n +15 | xargs -r rm --
echo "已备份到 $dest/backup-$stamp.sqlite3"
