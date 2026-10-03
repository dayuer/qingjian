# Mac 常驻程序

菜单栏里的剪贴板图标。本机复制的文本上传到你的服务器；别的设备刚复制的（2 分钟内）直接写进本机剪贴板，`⌘V` 就能贴；
更早的在菜单里列出，点一条复制到本机。它是独立的程序，不改输入法。

## 安装

```bash
cd cloud/mac-agent
scripts/bundle.sh --install     # 装到 ~/Applications/QingjianCloud.app，登录后自动启动
```

第一次运行会生成 `~/Library/Application Support/QingjianCloud/config.toml`。在菜单栏图标里点「打开配置文件…」，填上：

```toml
server = "https://你的域名"
token = "qjc_…"   # 服务器上 device add 打印的令牌
```

保存后点「重新加载配置」。卸载：`scripts/bundle.sh --uninstall`。

## 行为

- 带「密码」标记的复制（1Password、钥匙串访问等按 nspasteboard.org 约定标记的）不上传。
- 离线时复制的内容进本地队列（落盘，重启不丢），恢复网络后按顺序补发；菜单显示待上传条数，图标变灰，不弹通知。
- 「暂停同步」期间既不上传也不写入。
- 日志：`~/Library/Logs/QingjianCloud/`，按天保留 7 份。

## 第 1 期验收（真机）

两台 Mac 都装好、配好不同设备的令牌，然后：

1. A 上复制一段中文，B 上 1 秒内能 `⌘V` 贴出来；反过来也一样。
2. 在 1Password（或钥匙串访问「拷贝密码」）里复制密码，另一台收不到，服务器日志里也没有。
3. A 关掉 Wi-Fi，复制两段，菜单显示「离线 · 2 条待上传」、图标变灰；打字不受任何影响。打开 Wi-Fi 后 B 收到这两段。
4. A 合盖 5 分钟再打开，B 复制的内容 A 在 1 分钟内能收到。
5. B 上点菜单里的历史条目，内容进 B 的剪贴板，A 不会收到重复的一条。
