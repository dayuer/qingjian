#!/usr/bin/env bash
# 青简 Cloud 一键部署（在 VPS 上以 root 运行）。可重复运行：已装的跳过，代码拉到最新再重建。
#
#   curl -fsSL https://raw.githubusercontent.com/dayuer/qingjian/<分支>/cloud/deploy/install.sh -o install.sh
#   sudo DOMAIN=cloud.example.com CF_API_TOKEN=<令牌> DEVICES="macbook imac" bash install.sh
#
# 环境变量：
#   DOMAIN                 必填，服务的域名
#   CF_API_TOKEN           可选，Cloudflare API 令牌（需 Zone.DNS 编辑权限）；给了就自动把 DOMAIN 的 A 记录指向本机（仅 DNS，不走 CF 代理）
#   SERVER_IP              可选，本机公网 IP；不填自动探测
#   DEVICES                可选，空格分隔的设备名，没登记过的登记并打印令牌
#   QINGJIAN_LLM_API_KEY   可选，上游大模型密钥（填了才提供大模型代理）
#   ENABLE_TUNER=1         可选，同时启动纠错闭环（登记 tuner 设备、写令牌；缺省只出报告不推送，见 .env 的 QINGJIAN_TUNER_DRY_RUN）
#   REPO / BRANCH / DIR    代码来源与安装目录，缺省见下
set -euo pipefail

REPO="${REPO:-https://github.com/dayuer/qingjian}"
BRANCH="${BRANCH:-claude/gallant-brown-c1v3zi}"
DIR="${DIR:-/opt/qingjian}"
DOMAIN="${DOMAIN:?用法：DOMAIN=cloud.example.com bash install.sh}"

say() { printf '\n\033[1m==> %s\033[0m\n' "$*"; }
die() { printf '\033[31m错误：%s\033[0m\n' "$*" >&2; exit 1; }

[[ "$(id -u)" == 0 ]] || die "请用 root 运行（sudo）"

install_pkg() {
  if command -v apt-get >/dev/null; then
    DEBIAN_FRONTEND=noninteractive apt-get update -qq && DEBIAN_FRONTEND=noninteractive apt-get install -y -qq "$@"
  elif command -v dnf >/dev/null; then
    dnf install -y -q "$@"
  elif command -v yum >/dev/null; then
    yum install -y -q "$@"
  else
    die "不认识的包管理器，请手动安装：$*"
  fi
}

say "检查依赖"
command -v curl >/dev/null || install_pkg curl
command -v git >/dev/null || install_pkg git
command -v python3 >/dev/null || install_pkg python3
if ! command -v docker >/dev/null; then
  say "安装 Docker"
  curl -fsSL https://get.docker.com | sh
fi
systemctl enable --now docker >/dev/null 2>&1 || true
docker compose version >/dev/null 2>&1 || die "缺少 docker compose 插件"

IP="${SERVER_IP:-$(curl -fsS -4 -m 10 https://api.ipify.org || curl -fsS -4 -m 10 https://ifconfig.me)}"
[[ -n "$IP" ]] || die "探测不到公网 IP，请用 SERVER_IP 指定"
echo "本机公网 IP：$IP"

if [[ -n "${CF_API_TOKEN:-}" ]]; then
  say "Cloudflare：$DOMAIN → ${IP}（仅 DNS）"
  CF_API_TOKEN="$CF_API_TOKEN" DOMAIN="$DOMAIN" IP="$IP" python3 - <<'PY'
import json, os, sys, urllib.request

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

# 从最长的后缀往短找 zone：a.b.example.com → b.example.com → example.com
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
if existing:
    current = existing[0]
    if current["content"] == ip and not current["proxied"]:
        print("A 记录已是 %s，不用改" % ip)
    else:
        call("PUT", "/zones/%s/dns_records/%s" % (zone["id"], current["id"]), record)
        print("已更新 A 记录：%s → %s" % (domain, ip))
else:
    call("POST", "/zones/%s/dns_records" % zone["id"], record)
    print("已新建 A 记录：%s → %s" % (domain, ip))
PY
fi

say "等 $DOMAIN 解析到 $IP"
for _ in $(seq 1 36); do
  resolved="$(getent ahostsv4 "$DOMAIN" 2>/dev/null | awk 'NR==1{print $1}')"
  [[ "$resolved" == "$IP" ]] && break
  sleep 5
done
[[ "${resolved:-}" == "$IP" ]] && echo "已解析" || echo "还没解析到（当前：${resolved:-无}），Caddy 会自动重试申请证书，继续"

say "拉取代码 ${REPO}（${BRANCH}）到 $DIR"
if [[ -d "$DIR/.git" ]]; then
  git -C "$DIR" fetch -q --depth 1 origin "$BRANCH"
  git -C "$DIR" checkout -q -B "$BRANCH" FETCH_HEAD
else
  git clone -q --depth 1 -b "$BRANCH" "$REPO" "$DIR"
fi
cd "$DIR/cloud/deploy"

say "写配置 .env"
[[ -f .env ]] || cp .env.example .env
set_env() {
  local key="$1" value="$2"
  if grep -q "^$key=" .env; then
    sed -i "s|^$key=.*|$key=$value|" .env
  else
    echo "$key=$value" >> .env
  fi
}
set_env QINGJIAN_DOMAIN "$DOMAIN"
[[ -n "${QINGJIAN_LLM_API_KEY:-}" ]] && set_env QINGJIAN_LLM_API_KEY "$QINGJIAN_LLM_API_KEY"
chmod 600 .env

if command -v ufw >/dev/null && ufw status | grep -q "Status: active"; then
  ufw allow 80/tcp >/dev/null && ufw allow 443/tcp >/dev/null && echo "ufw 已放行 80 / 443"
elif command -v firewall-cmd >/dev/null && firewall-cmd --state >/dev/null 2>&1; then
  firewall-cmd -q --permanent --add-service=http --add-service=https && firewall-cmd -q --reload && echo "firewalld 已放行 80 / 443"
fi

say "构建并启动（第一次构建要几分钟）"
docker compose up -d --build

say "等 HTTPS 就绪"
ok=0
for _ in $(seq 1 60); do
  if curl -fsS -m 5 "https://$DOMAIN/healthz" >/dev/null 2>&1; then ok=1; break; fi
  sleep 5
done
if [[ "$ok" == 1 ]]; then
  echo "https://$DOMAIN/healthz 正常"
else
  echo "5 分钟内没等到 HTTPS。常见原因：云厂商安全组没放行 80 / 443、域名还没解析到本机。"
  echo "查看日志：cd $DIR/cloud/deploy && docker compose logs caddy cloud"
fi

for name in ${DEVICES:-}; do
  if docker compose exec -T cloud qingjian-cloud device list | cut -f1 | grep -qx "$name"; then
    echo "设备 $name 已登记过（令牌只在登记时显示；要换令牌先 device remove 再 add）"
  else
    say "登记设备 $name"
    docker compose exec -T cloud qingjian-cloud device add "$name"
  fi
done

if [[ "${ENABLE_TUNER:-}" == 1 ]]; then
  say "纠错闭环"
  if ! grep -q '^QINGJIAN_TUNER_TOKEN=qjc_' .env; then
    docker compose exec -T cloud qingjian-cloud device remove tuner >/dev/null 2>&1 || true
    token="$(docker compose exec -T cloud qingjian-cloud device add tuner | tail -1 | tr -d '\r')"
    set_env QINGJIAN_TUNER_TOKEN "$token"
  fi
  docker compose --profile tuner up -d --build tuner
  echo "已启动，每 24 小时一轮；每轮的报告打印在日志里：docker compose --profile tuner logs tuner"
fi

say "完成"
cat <<EOF
服务地址：https://$DOMAIN
管理命令都在 $DIR/cloud/deploy 下运行：
  docker compose exec cloud qingjian-cloud device add <设备名>
  docker compose exec cloud qingjian-cloud device list
  docker compose exec cloud qingjian-cloud usage
备份：crontab 加一行  0 4 * * * $DIR/cloud/deploy/backup.sh /root/qingjian-backups
升级：重新运行本脚本即可
EOF
