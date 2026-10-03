# 在自己的 VPS 上部署

需要：一台能装 Docker 的 VPS、一个解析到它的域名、放行 80 / 443 端口。

## 1. 启动

```bash
git clone https://github.com/<你>/qingjian.git && cd qingjian/cloud/deploy
cp .env.example .env          # 改 QINGJIAN_DOMAIN 为你的域名
docker compose up -d --build  # 第一次构建约几分钟；Caddy 自动申请 HTTPS 证书
curl https://<你的域名>/healthz  # 返回 ok 就好了
```

## 2. 登记设备

每台设备一个令牌，只显示一次：

```bash
docker compose exec cloud qingjian-cloud device add macbook
docker compose exec cloud qingjian-cloud device add imac
docker compose exec cloud qingjian-cloud device list
docker compose exec cloud qingjian-cloud device remove macbook   # 设备丢了：令牌立即失效
```

## 3. 备份与升级

```bash
crontab -e   # 每天 4 点备份，保留 14 份：
# 0 4 * * * /path/to/qingjian/cloud/deploy/backup.sh /path/to/backups

git pull && docker compose up -d --build   # 升级
```

数据在 Docker 命名卷 `deploy_cloud_data` 里（SQLite 单文件）。`docker compose down` 不会删数据，加 `-v` 才会。

## 可调的

`.env` 里：

| 变量 | 缺省 | 说明 |
|---|---|---|
| `QINGJIAN_DOMAIN` | 无，必填 | 域名；填 `:80` 则不启用 HTTPS（只建议在内网测试时用） |
| `QINGJIAN_CLOUD_CLIP_KEEP` | 200 | 剪贴板最多保留多少条 |
| `QINGJIAN_CLOUD_CLIP_DAYS` | 30 | 剪贴板最多保留多少天 |

## 接口一览

客户端都带 `Authorization: Bearer <令牌>`；协议类型见 `crates/qingjian-cloud-proto`。

| 方法与路径 | 说明 |
|---|---|
| `GET /healthz` | 健康检查，不要令牌 |
| `GET /v1/whoami` | 当前设备名与最新 `seq` |
| `POST /v1/clipboard` | 上传 `{"client_id", "text"}`；同一 `client_id` 重发只记一次 |
| `DELETE /v1/clipboard/{seq}` | 删除一条，所有设备同步删除 |
| `GET /v1/events?since=&limit=` | 拉 `since` 之后的事件 |
| `GET /v1/events/stream?since=` | SSE：先补积压，再推实时事件 |
