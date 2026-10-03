# 在自己的 VPS 上部署

需要：一台能装 Docker 的 VPS、一个解析到它的域名、放行 80 / 443 端口。

80 / 443 已经被宿主机的 nginx 占用（证书由 certbot 管）的机器，改用 `deploy-nginx.sh`：在本机运行，经 ssh 部署；
不启动 Caddy，cloud 只绑定本机端口，由 nginx 转发；同时申请证书、写好 nginx 站点、装每日备份。用法见脚本开头。

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

## 3. 大模型与输入日志

在 `.env` 里填 `QINGJIAN_LLM_API_KEY` 后 `docker compose up -d`，再在每台 Mac 的菜单栏图标里点「让青简使用 Cloud 的大模型」：
输入法的云联想改走服务器，Mac 上不再需要存大模型密钥。

```bash
docker compose exec cloud qingjian-cloud usage              # 最近 30 天按设备的请求数、缓存命中、token
docker compose exec cloud qingjian-cloud export-log > all.jsonl          # 所有设备的输入日志
docker compose exec cloud qingjian-cloud export-log --device macbook > mac.jsonl
```

导出的日志可以直接给青简的 `qingjian-cli --replay` 回放评测。

## 4. 纠错闭环（可选）

先配好上面的大模型密钥，然后：

```bash
ENABLE_TUNER=1 DOMAIN=<域名> bash install.sh     # 用部署脚本：登记 tuner 设备、写令牌、启动
# 或者手动：
docker compose exec cloud qingjian-cloud device add tuner   # 令牌填进 .env 的 QINGJIAN_TUNER_TOKEN
docker compose --profile tuner up -d --build
docker compose --profile tuner logs -f tuner                # 每轮的报告
```

缺省只出报告不推送（`.env` 的 `QINGJIAN_TUNER_DRY_RUN=true`）；看几轮报告觉得靠谱，改成 `false` 再 `docker compose --profile tuner up -d`。
镜像约 230 MB（带上游的 qingjian-cli 与产品数据），第一次构建要编译上游并下载数据，十分钟左右。

## 5. 备份与升级

```bash
crontab -e   # root 的 crontab，每天 4 点备份，保留 14 份：
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
| `QINGJIAN_LLM_API_KEY` | 空 | 上游大模型密钥；填了才提供大模型代理 |
| `QINGJIAN_LLM_BASE_URL` | `https://api.deepseek.com` | 上游接口地址，不含 `/chat/completions`（OpenAI 填 `https://api.openai.com/v1`） |
| `QINGJIAN_LLM_MODEL` | 空 | 覆盖输入法请求里的模型名 |
| `QINGJIAN_LLM_CONTEXT_CHARS` | 0 | 往请求里插多少字「各设备最近输入的文字」当上文；0 为不插 |

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
| `POST /v1/learning`、`GET /v1/learning?since=` | 学习数据：推变化、拉合并后的当前值 |
| `GET` / `PUT /v1/config` | 输入法的 `config.toml`（带版本号，冲突返回 409） |
| `POST /v1/input-log`、`GET /v1/input-log?since=` | 输入日志：上传一批（按批号去重）、拉所有设备的 |
| `POST /v1/input-log/clear` | 清空所有设备的输入日志 |
| `POST /v1/chat/completions` | 大模型代理（OpenAI 兼容），令牌换成服务器的密钥后转给上游 |
