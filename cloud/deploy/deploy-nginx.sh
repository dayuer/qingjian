#!/usr/bin/env bash
# 青简 Cloud 部署到已有 nginx + certbot 的 VPS（43.156.128.95）。在本机运行，可重复运行。
#
#   CF_API_TOKEN=<令牌> DEVICES="macbook" cloud/deploy/deploy-nginx.sh
#
# 与 install.sh 的区别：那台机器的 80/443 归宿主机 nginx，证书由 certbot 统一续期
# （/opt/ssl-renew.sh），所以不起 compose 里的 Caddy，cloud 只绑 127.0.0.1，由 nginx 反代。
# Cloudflare 令牌只在本机用来改 DNS，不传到服务器。
set -euo pipefail

HOST="${HOST:-root@43.156.128.95}"
SERVER_IP="${SERVER_IP:-43.156.128.95}"
DOMAIN="${DOMAIN:-pingyin.synon.ai}"
PORT="${PORT:-18100}"
REPO="${REPO:-https://github.com/dayuer/qingjian}"
BRANCH="${BRANCH:-claude/gallant-brown-c1v3zi}"
DEVICES="${DEVICES:-}"

say() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }

if [[ -n "${CF_API_TOKEN:-}" ]]; then
  say "Cloudflare：$DOMAIN → ${SERVER_IP}（仅 DNS）"
  CF_API_TOKEN="$CF_API_TOKEN" DOMAIN="$DOMAIN" IP="$SERVER_IP" python3 - <<'PY'
import json, os, sys, urllib.error, urllib.request

token, domain, ip = os.environ["CF_API_TOKEN"], os.environ["DOMAIN"], os.environ["IP"]

def call(method, path, body=None):
    request = urllib.request.Request(
        "https://api.cloudflare.com/client/v4" + path,
        method=method,
        data=None if body is None else json.dumps(body).encode(),
        headers={"Authorization": "Bearer " + token, "Content-Type": "application/json"},
    )
    try:
        with urllib.request.urlopen(request, timeout=20) as response:
            data = json.load(response)
    except urllib.error.HTTPError as error:
        data = json.load(error)
    if not data.get("success"):
        sys.exit("Cloudflare API 失败：%s" % data.get("errors"))
    return data["result"]

labels = domain.split(".")
zone = None
for i in range(len(labels) - 1):
    found = call("GET", "/zones?name=" + ".".join(labels[i:]))
    if found:
        zone = found[0]
        break
if zone is None:
    sys.exit("这个令牌下找不到 %s 所在的 zone" % domain)

record = {"type": "A", "name": domain, "content": ip, "ttl": 1, "proxied": False}
existing = call("GET", "/zones/%s/dns_records?type=A&name=%s" % (zone["id"], domain))
if existing and existing[0]["content"] == ip and not existing[0]["proxied"]:
    print("A 记录已是 %s，不用改" % ip)
elif existing:
    call("PUT", "/zones/%s/dns_records/%s" % (zone["id"], existing[0]["id"]), record)
    print("已更新 A 记录：%s → %s" % (domain, ip))
else:
    call("POST", "/zones/%s/dns_records" % zone["id"], record)
    print("已新建 A 记录：%s → %s" % (domain, ip))
PY
fi

say "等 $DOMAIN 在公共 DNS 上解析到 $SERVER_IP"
for _ in $(seq 1 36); do
  [[ "$(dig +short @1.1.1.1 "$DOMAIN" A | tail -1)" == "$SERVER_IP" ]] && { echo "已解析"; break; }
  sleep 5
done

say "远端部署（${HOST}）"
ssh "$HOST" DOMAIN="$DOMAIN" PORT="$PORT" REPO="$REPO" BRANCH="$BRANCH" DEVICES="'$DEVICES'" bash -s <<'REMOTE'
set -euo pipefail
say() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }

DIR=/opt/qingjian
HOSTCFG=/opt/qingjian-host
SITE=/etc/nginx/sites-available/$DOMAIN

say "拉取代码 $BRANCH → $DIR"
if [[ -d "$DIR/.git" ]]; then
  git -C "$DIR" fetch -q --depth 1 origin "$BRANCH"
  git -C "$DIR" checkout -q -B "$BRANCH" FETCH_HEAD
else
  git clone -q --depth 1 -b "$BRANCH" "$REPO" "$DIR"
fi
git -C "$DIR" log --oneline -1

say "本机专用的 compose 覆盖（${HOSTCFG}）"
mkdir -p "$HOSTCFG" "$HOSTCFG/releases"
# 原 Dockerfile 加一行 CARGO_BUILD_JOBS=1：2 核 3.7G 的机器上跑着线上服务，构建慢点也不能挤内存
sed '/^WORKDIR \/src/a ENV CARGO_BUILD_JOBS=1' "$DIR/cloud/server/Dockerfile" > "$HOSTCFG/Dockerfile"
cat > "$HOSTCFG/compose.nginx.yml" <<EOF
# 由 deploy-nginx.sh 生成。80/443 归宿主机 nginx：cloud 只绑本机端口，Caddy 放进不启用的 profile。
services:
  cloud:
    build:
      dockerfile: $HOSTCFG/Dockerfile
    ports:
      - "127.0.0.1:$PORT:8080"
  caddy:
    profiles: ["caddy"]
EOF

cd "$DIR/cloud/deploy"
[[ -f .env ]] || cp .env.example .env
set_env() {
  if grep -q "^$1=" .env; then sed -i "s|^$1=.*|$1=$2|" .env; else echo "$1=$2" >> .env; fi
}
set_env QINGJIAN_DOMAIN "$DOMAIN"
# 写进 .env 后，在这个目录下直接 docker compose … 就会带上覆盖文件
set_env COMPOSE_FILE "docker-compose.yml:$HOSTCFG/compose.nginx.yml"
chmod 600 .env
docker compose config --services

say "构建并启动 cloud（第一次构建要十几分钟）"
docker compose up -d --build cloud
for _ in $(seq 1 30); do
  curl -fsS -m 3 "http://127.0.0.1:$PORT/healthz" >/dev/null 2>&1 && break
  sleep 2
done
echo "本机 healthz：$(curl -fsS -m 3 "http://127.0.0.1:$PORT/healthz")"

write_http_only() {
  cat > "$SITE" <<EOF
# 青简 Cloud（/opt/qingjian），由 deploy-nginx.sh 生成
server {
    listen 80;
    listen [::]:80;
    server_name $DOMAIN;
    location /.well-known/acme-challenge/ { root /var/www/certbot; }
    location / { return 301 https://\$host\$request_uri; }
}
EOF
}

write_full() {
  write_http_only
  cat >> "$SITE" <<EOF

server {
    listen 443 ssl http2;
    listen [::]:443 ssl http2;
    server_name $DOMAIN;

    ssl_certificate /etc/letsencrypt/live/$DOMAIN/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/$DOMAIN/privkey.pem;
    include /etc/letsencrypt/options-ssl-nginx.conf;
    ssl_dhparam /etc/letsencrypt/ssl-dhparams.pem;

    location /.well-known/acme-challenge/ { root /var/www/certbot; }

    # 输入日志与 config.toml 是整批上传
    client_max_body_size 20m;

    # 自建更新：版本索引、签名与安装包（cloud/scripts/publish-mac.sh 上传），目录页就是下载页
    location /releases/ {
        alias /opt/qingjian-host/releases/;
        autoindex on;
        location ~ \.(json|sig)\$ { add_header Cache-Control "no-cache"; }
    }

    location / {
        proxy_pass http://127.0.0.1:$PORT;
        proxy_http_version 1.1;
        proxy_set_header Connection "";
        proxy_set_header Host \$host;
        proxy_set_header X-Real-IP \$remote_addr;
        proxy_set_header X-Forwarded-For \$proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto \$scheme;
        # /v1/events/stream 是 SSE，要逐条转发、长时间不断
        proxy_buffering off;
        proxy_cache off;
        proxy_read_timeout 1h;
        proxy_send_timeout 1h;
    }
}
EOF
}

reload_nginx() { nginx -t && systemctl reload nginx; }

say "nginx 站点 $DOMAIN"
if [[ ! -f "/etc/letsencrypt/live/$DOMAIN/fullchain.pem" ]]; then
  write_http_only
  ln -sfn "$SITE" "/etc/nginx/sites-enabled/$DOMAIN"
  reload_nginx
  say "申请证书（webroot，续期并入 /opt/ssl-renew.sh 的 certbot renew）"
  certbot certonly --webroot -w /var/www/certbot -d "$DOMAIN" --non-interactive --keep-until-expiring
fi
write_full
ln -sfn "$SITE" "/etc/nginx/sites-enabled/$DOMAIN"
reload_nginx

say "验证 HTTPS"
# systemctl reload 不等新 worker 接管就返回，立刻请求会被兜底 server 拒握手
for _ in $(seq 1 10); do
  curl -fsS -m 10 "https://$DOMAIN/healthz" 2>/dev/null && { echo; break; }
  sleep 2
done

# 本脚本经 ssh 的 stdin 送过来，exec 都要接 /dev/null，否则会把后面的脚本当输入吃掉
for name in $DEVICES; do
  if docker compose exec -T cloud qingjian-cloud device list </dev/null | cut -f1 | grep -qx "$name"; then
    echo "设备 $name 已登记过（令牌只在登记时显示；要换先 device remove 再 add）"
  else
    say "登记设备 ${name}（令牌只显示这一次）"
    docker compose exec -T cloud qingjian-cloud device add "$name" </dev/null
  fi
done

say "每日备份（04:30，留 14 份，/root/qingjian-backups）"
# cloud/deploy/backup.sh 最后一步用容器里的 rm，distroless 镜像没有，所以用宿主机这份
cat > "$HOSTCFG/backup.sh" <<'SH'
#!/usr/bin/env bash
# 青简 Cloud 每日备份（由 deploy-nginx.sh 生成）。cloud/deploy/backup.sh 最后用容器里的 rm 删临时文件，
# 但镜像是 distroless 没有 rm；这里改成从宿主机的卷目录直接移走。
set -euo pipefail
dest="${1:-/root/qingjian-backups}"
mkdir -p "$dest" && chmod 700 "$dest"
stamp="$(date +%Y%m%d-%H%M%S)"
vol="$(docker volume inspect deploy_cloud_data --format '{{.Mountpoint}}')"
cd /opt/qingjian/cloud/deploy
docker compose exec -T cloud qingjian-cloud backup "/data/backup-$stamp.sqlite3" </dev/null >/dev/null
mv "$vol/backup-$stamp.sqlite3" "$dest/"
chown root:root "$dest/backup-$stamp.sqlite3" && chmod 600 "$dest/backup-$stamp.sqlite3"
# 只留最近 14 份
ls -1t "$dest"/backup-*.sqlite3 | tail -n +15 | xargs -r rm --
echo "$(date '+%F %T') 已备份到 $dest/backup-$stamp.sqlite3"
SH
chmod 700 "$HOSTCFG/backup.sh"
# 04:00 / 04:10 已有别的任务，错开
( crontab -l 2>/dev/null | grep -v qingjian-host/backup.sh
  echo "30 4 * * * $HOSTCFG/backup.sh /root/qingjian-backups >> /var/log/qingjian-backup.log 2>&1" ) | crontab -

say "完成：https://$DOMAIN"
echo "管理命令在 $DIR/cloud/deploy 下直接 docker compose exec cloud qingjian-cloud …"
REMOTE
