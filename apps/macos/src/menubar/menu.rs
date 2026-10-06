use objc2::rc::Retained;
use objc2::{MainThreadMarker, sel};
use objc2_app_kit::{NSMenu, NSMenuItem};
use objc2_foundation::NSString;
use qingjian_platform::Config;

use super::MenuAction;
use super::cloud_agent::CloudAgentMenu;
use super::target::MenuTarget;

/// 菜单本体与需要按状态刷新的那几项。
pub struct InputMenu {
    /// 菜单。挂到状态项和 IMK `menu` 回调的是同一个对象。
    menu: Retained<NSMenu>,

    /// 配置文件解析失败时显示的提示行，平时隐藏。
    error: Retained<NSMenuItem>,

    /// 「有新版本 x.y.z…」，点了打开下载页；没有新版时隐藏。
    update: Retained<NSMenuItem>,

    /// 「素笺云 ›」子菜单；没装素笺云时隐藏。
    cloud_agent: CloudAgentMenu,

    /// 所有条目的 target，要和菜单活得一样久。
    _target: Retained<MenuTarget>,
}

impl InputMenu {
    pub fn new(mtm: MainThreadMarker, version: &str) -> Self {
        let target = MenuTarget::new(mtm);
        let menu = NSMenu::new(mtm);
        // 不让 AppKit 按响应链判断可用性：它找不到 target 就会把整份菜单灰掉
        menu.setAutoenablesItems(false);

        // 云联想与模糊音不再放顶层：云联想的开关在「素笺云 ›」里（跟其他云功能一处），
        // 模糊音偏好设置里已有。素笺云父项排在第一个，前面不能有隐藏项或分隔线，见 cloud_agent.rs 文件头
        let cloud_agent = CloudAgentMenu::new(mtm, &target);
        menu.addItem(cloud_agent.item());

        menu.addItem(&NSMenuItem::separatorItem(mtm));
        menu.addItem(&action_item(
            mtm,
            "偏好设置…",
            Some(MenuAction::OpenPreferences),
            &target,
        ));
        menu.addItem(&action_item(
            mtm,
            "打开日志目录",
            Some(MenuAction::OpenLogs),
            &target,
        ));
        // 可点的条目只能放在这一组：放到下面两个纯展示条目之间，IMK 会在每次按键后停用再新建会话，打不了字
        let update = action_item(mtm, "", Some(MenuAction::OpenDownload), &target);
        update.setHidden(true);
        menu.addItem(&update);
        menu.addItem(&NSMenuItem::separatorItem(mtm));

        let error = action_item(mtm, "", None, &target);
        error.setEnabled(false);
        error.setHidden(true);
        menu.addItem(&error);
        let about = action_item(mtm, &format!("素笺 {version}"), None, &target);
        about.setEnabled(false);
        menu.addItem(&about);

        Self {
            menu,
            error,
            update,
            cloud_agent,
            _target: target,
        }
    }

    /// 每秒一次：照素笺云给的菜单行刷新「素笺云」子菜单。
    pub fn sync_cloud_agent(&self, mtm: MainThreadMarker) {
        self.cloud_agent.sync(mtm, &self._target);
    }

    /// 查到新版本就露出「有新版本」那一行，没有就藏起来。
    pub fn sync_update(&self, available: Option<&str>) {
        match available {
            Some(version) => {
                self.update
                    .setTitle(&NSString::from_str(&format!("有新版本 {version}…")));
                self.update.setHidden(false);
            }
            None => self.update.setHidden(true),
        }
    }

    /// 给状态项 / IMK 回调用的菜单对象。
    pub fn ns_menu(&self) -> Retained<NSMenu> {
        self.menu.clone()
    }

    /// 按当前配置刷新勾选状态。`cloud_active` 是 Engine 里真接上了 Predictor：
    /// 配置开了但没接上（多半是没密钥）时不打勾，标题说明原因，不能显示开了实际没开。
    /// `notice` 是配置文件的问题（`Settings::notice()` 已写好措辞），原样显示，没有问题时藏起来。
    pub fn sync(&self, _config: &Config, _cloud_active: bool, notice: Option<&str>) {
        match notice {
            Some(message) => {
                self.error.setTitle(&NSString::from_str(message));
                self.error.setHidden(false);
            }
            None => self.error.setHidden(true),
        }
    }
}

/// 建一个菜单项。`action` 为 `None` 的是纯展示项（子菜单父项、关于行）。
pub(super) fn action_item(
    mtm: MainThreadMarker,
    title: &str,
    action: Option<MenuAction>,
    target: &MenuTarget,
) -> Retained<NSMenuItem> {
    // SAFETY: 选择器与 MenuTarget / 控制器上定义的 `menuAction:` 一致，签名 (id) -> void
    let item = unsafe {
        NSMenuItem::initWithTitle_action_keyEquivalent(
            mtm.alloc(),
            &NSString::from_str(title),
            action.map(|_| sel!(menuAction:)),
            &NSString::from_str(""),
        )
    };
    if let Some(action) = action {
        unsafe { item.setTarget(Some(target)) };
        item.setTag(action.tag());
    }
    item
}
