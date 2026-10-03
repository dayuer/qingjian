# Mac 常驻程序

后台常驻，不占菜单栏：状态与操作在输入法「中☁」菜单的「青简 Cloud ›」子菜单里（配置 `menu_bar = true` 可另显示自己的图标）。
**只在用青简时运行**：切到别的输入法 30 秒后自己退出，切回青简时由输入法拉起；所以用别的输入法时剪贴板也不同步。做这些事：

- **剪贴板**：本机复制的文本上传到你的服务器；别的设备刚复制的（2 分钟内）直接写进本机剪贴板，`⌘V` 就能贴；更早的在菜单里列出，点一条复制到本机。
- **学习数据与设置**：词频、选择、n-gram、敲错表、英文词、用户词，以及 `config.toml`（设置与自定义短语）在各台 Mac 之间合并。
  需要装打了 Cloud 补丁的青简（`local` 分支，见下）；装的是官方版时菜单会一直显示「等输入法合并」，不会出错也不会改坏数据。
- **输入日志**：输入法的 `input-log.jsonl` 上传到服务器；别的设备的下载到 `~/Library/Application Support/QingjianCloud/input-log/`。
- **大模型**：菜单里「让青简使用 Cloud 的大模型」把输入法的云联想改走你的服务器，Mac 上不再需要大模型密钥（服务器要先配好 `QINGJIAN_LLM_API_KEY`）。

## 安装

先装打了补丁的输入法（学习数据同步要用；只要剪贴板可以跳过）：

```bash
BASE=origin/claude/gallant-brown-c1v3zi cloud/scripts/build-local.sh   # 生成 local 分支
git checkout local && apps/macos/scripts/bundle.sh --install
```

再装常驻程序：

```bash
cd cloud/mac-agent
scripts/bundle.sh --install     # 装到 ~/Applications/QingjianCloud.app，登录后自动启动
```

第一次运行会生成 `~/Library/Application Support/QingjianCloud/config.toml`。在「中☁ → 青简 Cloud」里点「打开配置文件…」，填上：

```toml
server = "https://你的域名"
token = "qjc_…"   # 服务器上 device add 打印的令牌
```

保存后点「重新加载配置」。从输入法 pkg 安装（`cloud/scripts/publish-mac.sh` 发的包）时已一起装好。卸载：`scripts/bundle.sh --uninstall`。

## 行为

- 带「密码」标记的复制（1Password、钥匙串访问等按 nspasteboard.org 约定标记的）不上传。
- 离线时复制的内容进本地队列（落盘，重启不丢），恢复网络后按顺序补发；菜单显示待上传条数，图标变灰，不弹通知。
- 「暂停同步」期间既不上传也不写入。
- 日志：`~/Library/Logs/QingjianCloud/`，按天保留 7 份。

## 学习数据与设置的验收（真机）

两台 Mac 都装好补丁版输入法与常驻程序：

1. A 上反复选某个非首选的词（比如打 `shiguo` 选第三个），菜单「立即同步学习数据」，B 上也点一下，B 打同样的拼音时它已经排前面。
2. A 上造一个新词（云联想选中的、或自动造的），B 能直接打出来；B 上按删除修饰键删掉它，A 上也没了。
3. A 的偏好设置改双拼方案、加一条自定义短语，B 半分钟内生效。
4. 两台断网各自打字，恢复后都同步，词频是两边相加（看 `user.tsv`），不是二选一，也没有翻倍。
5. 菜单显示「等输入法合并」时切到任意文本框打几个字，几秒后变成「刚刚同步」。

## 大模型与输入日志的验收（真机）

1. 服务器 `.env` 填好 `QINGJIAN_LLM_API_KEY` 并重启；Mac 菜单点「让青简使用 Cloud 的大模型」。
   青简「偏好设置 → 云服务」里地址变成 `https://<服务器>/v1`，点「测试连接」成功；打一段拼音停一下，候选末尾出现云端词。
2. 删掉 Mac 上原来的 DeepSeek 密钥（`.env` 里的 `QINGJIAN_API_KEY`）后，云联想照常工作。
3. 服务器上 `qingjian-cloud usage` 能看到这台设备的请求数与 token。
4. 打几句话，半分钟后服务器上 `qingjian-cloud export-log --device <设备名>` 能看到这些上屏；另一台 Mac 的
   `~/Library/Application Support/QingjianCloud/input-log/<设备名>.jsonl` 里也有。
5. 青简「偏好设置 → 高级 → 清空输入日志」后，服务器上 `export-log` 为空，另一台下载的副本也被删掉。
6. 断网打字，打字不受影响；恢复后日志补传。

## 第 1 期验收（真机）

两台 Mac 都装好、配好不同设备的令牌，然后：

1. A 上复制一段中文，B 上 1 秒内能 `⌘V` 贴出来；反过来也一样。
2. 在 1Password（或钥匙串访问「拷贝密码」）里复制密码，另一台收不到，服务器日志里也没有。
3. A 关掉 Wi-Fi，复制两段，菜单显示「离线 · 2 条待上传」、图标变灰；打字不受任何影响。打开 Wi-Fi 后 B 收到这两段。
4. A 合盖 5 分钟再打开，B 复制的内容 A 在 1 分钟内能收到。
5. B 上点菜单里的历史条目，内容进 B 的剪贴板，A 不会收到重复的一条。
