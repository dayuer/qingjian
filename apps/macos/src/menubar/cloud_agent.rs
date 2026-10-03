//! 「青简 Cloud ›」子菜单：青简 Cloud 常驻程序不占菜单栏，把菜单写成 `QingjianCloud/menu.txt`，这里照着画；
//! 点了哪项就往 `QingjianCloud/commands/` 写一个只含 tag 的文件，由它取走执行。两边只靠这两个文件通信，
//! 输入法不碰剪贴板与同步。格式见 `cloud/mac-agent/src/menu/lines.rs`。自用分叉补丁，见 `cloud/docs/fork-patch.md`。

use std::cell::Cell;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSMenu, NSMenuItem};

use super::MenuAction;
use super::menu::action_item;
use super::target::MenuTarget;

/// `menu.txt` 超过这么久没更新就当青简 Cloud 没在运行（它至少每分钟重写一次）。
const STALE_AFTER: Duration = Duration::from_secs(5 * 60);

/// 青简 Cloud 在 launchd 里的标签（`cloud/mac-agent/scripts/install-app.sh`）。
const AGENT_LABEL: &str = "app.qingjian.cloud.agent";

/// 拉起青简 Cloud 最多这么久试一次：activateServer 每切一次应用就来一次。
const START_THROTTLE_SECS: u64 = 10;

/// 上次尝试拉起的时间（Unix 秒）。
static LAST_START: AtomicU64 = AtomicU64::new(0);

pub struct CloudAgentMenu {
    /// 挂在输入法菜单上的父项；没装青简 Cloud（没有 `menu.txt`）时隐藏且不挂子菜单。
    /// 坑：父项挂着**空**子菜单时，IMK 整理菜单（`_copySynchronizedActions:withMenuItems:`）会 CFRelease(NULL)
    /// 直接崩溃，进程一起来就在 activateServer 里死，输入法打不了字。所以子菜单只在有内容时才挂上。
    item: Retained<NSMenuItem>,

    /// 上次画的 `menu.txt` 修改时间与是否已过期，都没变就不重画。
    drawn: Cell<Option<(SystemTime, bool)>>,
}

impl CloudAgentMenu {
    pub fn new(mtm: MainThreadMarker, target: &MenuTarget) -> Self {
        let item = action_item(mtm, "青简 Cloud", None, target);
        item.setHidden(true);
        Self {
            item,
            drawn: Cell::new(None),
        }
    }

    pub fn item(&self) -> &NSMenuItem {
        &self.item
    }

    /// 每秒一次：`menu.txt` 变了（或刚过期）才重画子菜单。
    pub fn sync(&self, mtm: MainThreadMarker, target: &MenuTarget) {
        let Some(path) = menu_file() else {
            return;
        };
        let Ok(modified) = std::fs::metadata(&path).and_then(|meta| meta.modified()) else {
            if self.drawn.take().is_some() {
                self.item.setHidden(true);
                self.item.setSubmenu(None);
            }
            return;
        };
        let stale = modified.elapsed().is_ok_and(|age| age > STALE_AFTER);
        if self.drawn.get() == Some((modified, stale)) {
            return;
        }
        self.drawn.set(Some((modified, stale)));
        // 每次新建一份再整个换上，保证挂上去的子菜单至少有一项
        let submenu = NSMenu::new(mtm);
        submenu.setAutoenablesItems(false);
        if !stale {
            let text = std::fs::read_to_string(&path).unwrap_or_default();
            for line in text.lines().filter(|line| !line.is_empty()) {
                submenu.addItem(&parse_line(mtm, line, target));
            }
        }
        if submenu.numberOfItems() == 0 {
            let note = if stale {
                "青简 Cloud 没在运行（切回青简会自动拉起）"
            } else {
                "青简 Cloud 正在启动…"
            };
            let item = action_item(mtm, note, None, target);
            item.setEnabled(false);
            submenu.addItem(&item);
        }
        self.item.setSubmenu(Some(&submenu));
        self.item.setHidden(false);
    }
}

fn parse_line(mtm: MainThreadMarker, line: &str, target: &MenuTarget) -> Retained<NSMenuItem> {
    if line == "---" {
        return NSMenuItem::separatorItem(mtm);
    }
    let (tag, title) = line.split_once('\t').unwrap_or(("-", line));
    match tag.parse::<isize>().ok().map(MenuAction::CloudAgent) {
        Some(action) if MenuAction::from_tag(action.tag()) == Some(action) => {
            action_item(mtm, title, Some(action), target)
        }
        _ => {
            let item = action_item(mtm, title, None, target);
            item.setEnabled(false);
            item
        }
    }
}

/// 子菜单里点了一项：交给青简 Cloud。文件名用纳秒时间，它按文件名顺序执行。
pub fn send_command(tag: isize) {
    let Some(dir) = cloud_dir().map(|dir| dir.join("commands")) else {
        return;
    };
    let name = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    let result = std::fs::create_dir_all(&dir)
        .and_then(|()| std::fs::write(dir.join(format!("{name}.cmd")), tag.to_string()));
    if let Err(error) = result {
        tracing::warn!(%error, "青简 Cloud 命令写入失败");
    }
}

/// 切到青简时调：青简 Cloud 装了（有 launchd 登记）却没在运行（它退出时删 `menu.txt`）就拉起它。
/// 它只在用青简时运行，切到别的输入法 30 秒后自己退出，见 `cloud/mac-agent/src/app.rs`。
pub fn ensure_agent_running() {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return;
    };
    let installed = home
        .join(format!("Library/LaunchAgents/{AGENT_LABEL}.plist"))
        .is_file();
    if !installed || menu_file().is_some_and(|path| path.is_file()) {
        return;
    }
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs());
    if now.saturating_sub(LAST_START.swap(now, Ordering::Relaxed)) < START_THROTTLE_SECS {
        return;
    }
    tracing::info!("拉起青简 Cloud");
    if let Err(error) = std::process::Command::new("launchctl")
        .args(["start", AGENT_LABEL])
        .spawn()
    {
        tracing::warn!(%error, "拉起青简 Cloud 失败");
    }
}

fn cloud_dir() -> Option<PathBuf> {
    Some(PathBuf::from(std::env::var_os("HOME")?).join("Library/Application Support/QingjianCloud"))
}

fn menu_file() -> Option<PathBuf> {
    Some(cloud_dir()?.join("menu.txt"))
}
