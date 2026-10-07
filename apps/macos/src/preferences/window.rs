//! 偏好设置窗口本体：把各页（`pages/`）装进标签视图，底部一行状态；刷新时逐页同步。

use std::cell::{Cell, RefCell};

use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSClipView, NSColor, NSScreen, NSScrollView, NSTabView, NSTabViewDelegate, NSTabViewItem,
    NSTextField, NSView,
};
use objc2_foundation::{NSObject, NSObjectProtocol, NSPoint, NSRect, NSSize, NSString};
use qingjian_core::{Language, UsageSummary, VocabularySummary};
use qingjian_platform::Config;
use qingjian_predict::PredictProvider;

use super::controls::{language_label, small_label};
use super::flipped::{self, FlippedView};
use super::layout::{Layout, PAGE_PADDING, PAGE_WIDTH};
use super::pages::{
    AboutPage, AdvancedPage, CandidatesPage, CloudPage, CloudShape, DictionariesPage, FuzzyPage,
    GeneralPage, PhrasesPage, ShortcutsPage, UpdateStatus, UsagePage, build_about,
};
use super::panel::PreferencesPanel;
use super::target::PreferencesTarget;
use crate::host::DictionaryInfo;

/// 每页的顶部与底部留白。
const PAGE_TOP: f64 = 16.0;

/// 标签视图四周留白、底部状态行（配置文件出错时显示原因）的高度与它下面的留白。
const TAB_MARGIN: f64 = 14.0;
const STATUS_HEIGHT: f64 = 16.0;
const STATUS_GAP: f64 = 6.0;

/// 窗口比屏幕可用高度至少矮这么多（标题栏 + 上下留一点边）；页面比窗口高时自己滚。
const SCREEN_MARGIN: f64 = 80.0;

/// 设置窗口与需要按配置刷新的各页。
pub struct PreferencesWindow {
    /// 窗口。
    panel: Retained<PreferencesPanel>,

    /// 「通用」页。
    general: GeneralPage,

    /// 「候选窗口」页。
    candidates: CandidatesPage,

    /// 「快捷键」页。
    shortcuts: ShortcutsPage,

    /// 自定义短语编辑。
    phrases: PhrasesPage,

    /// 「模糊音」页。
    fuzzy: FuzzyPage,

    /// 「词库」页。
    dictionaries: DictionariesPage,

    /// 「云服务」页（形状变了整页换掉，所以装在 `RefCell` 里）。
    cloud: RefCell<CloudPage>,

    /// 标签视图；「云服务设置…」要切到云服务页，窗口高度也按它的当前页算。
    tabs: Retained<NSTabView>,

    /// 换页签的代理：窗口高度要跟着新页面走（见 [`fit_window_to_page`]）。
    _tab_change: Retained<TabChange>,

    /// 「云服务」页在标签里的下标。
    cloud_tab: usize,

    /// 云服务页现在排成什么样；内容变了（开通 / 设备台数 / 展开高级）就整页重建。
    cloud_shape: Cell<CloudShape>,

    /// 最近一次按配置刷云服务页用的参数：重建之后新控件要按它填一遍。
    cloud_config: RefCell<Option<CloudConfig>>,

    /// 标签页区的高度（各页都按它封顶，云服务页也不矮过它）。
    page_height: f64,

    /// 「高级」页。
    advanced: AdvancedPage,

    /// 「统计」页的数字。
    usage: UsagePage,

    /// 「关于」页的检查更新控件。
    about: AboutPage,

    /// 底部状态行：配置文件解析失败时显示原因，也给临时提示用。
    status: Retained<NSTextField>,

    /// 所有控件的 target，要和窗口活得一样久。
    _target: Retained<PreferencesTarget>,
}

/// 现下的云状态：debug 构建下自检（`QJ_SETTINGS_FAKE`）能换成假的那一份，正式构建照实取。
fn cloud_status() -> qingjian_cloud_mac::CloudStatus {
    #[cfg(debug_assertions)]
    if std::env::var_os("QJ_SETTINGS_FAKE").is_some() {
        return fake_cloud_status();
    }
    qingjian_cloud_mac::status()
}

/// 截一张窗口（自检用，只在 debug 构建里）。
#[cfg(debug_assertions)]
fn shoot(number: isize, path: &str) {
    let _ = std::process::Command::new("screencapture")
        .args(["-x", "-o", "-l", &number.to_string()])
        .arg(path)
        .status();
}

/// 起手那张云服务页的形状：debug 构建下自检可以换成假状态（见 `fake_cloud_status`），
/// 正式构建永远是没开通的起手样子。
fn dev_shape() -> CloudShape {
    #[cfg(debug_assertions)]
    match std::env::var("QJ_SETTINGS_FAKE").as_deref() {
        Ok("0") => return CloudShape::closed(),
        Ok("1") => return shape_of(&fake_cloud_status(), false, false),
        Ok("2") => return shape_of(&fake_cloud_status(), true, true),
        _ => {}
    }
    CloudShape::closed()
}

/// 状态 + 换没换服务 + 展开与否 → 页面形状。
fn shape_of(status: &qingjian_cloud_mac::CloudStatus, custom: bool, advanced: bool) -> CloudShape {
    CloudShape {
        signed_in: status.signed_in,
        others: status
            .devices
            .iter()
            .filter(|device| !device.current)
            .count(),
        custom,
        advanced,
        note: status.note.as_deref().is_some_and(|note| !note.is_empty()),
    }
}

/// 开发自检（只在 debug 构建里）：`QJ_SETTINGS_FAKE=1` 时装的已开通状态，
/// 好把界面（设备行、危险按钮、红字）看全。
#[cfg(debug_assertions)]
fn fake_cloud_status() -> qingjian_cloud_mac::CloudStatus {
    if std::env::var("QJ_SETTINGS_FAKE").as_deref() == Ok("0") {
        return qingjian_cloud_mac::CloudStatus::empty();
    }
    qingjian_cloud_mac::CloudStatus {
        signed_in: true,
        line: "已开通".to_owned(),
        device: "你的 MacBook Pro".to_owned(),
        sync_line: "学习数据：3 分钟前同步".to_owned(),
        joining: false,
        paused: false,
        note: Some("解绑没成功：网络不通".to_owned()),
        devices: vec![
            qingjian_cloud_mac::CloudDevice {
                id: 7,
                name: "你的 iPhone 17 Pro Max 特别版".to_owned(),
                detail: "iOS · 3 分钟前活跃".to_owned(),
                current: false,
            },
            qingjian_cloud_mac::CloudDevice {
                id: 9,
                name: "iPad".to_owned(),
                detail: "iOS · 2 小时前活跃".to_owned(),
                current: false,
            },
            qingjian_cloud_mac::CloudDevice {
                id: 3,
                name: "这台 Mac".to_owned(),
                detail: "Mac · 刚刚活跃".to_owned(),
                current: true,
            },
        ],
        consents: [
            ("memory", true),
            ("input_log", false),
            ("sync", true),
            ("clipboard", false),
            ("llm", true),
        ],
    }
}

/// 一页：标题、布局器、承载视图。
type Page = (&'static str, Layout, Retained<NSView>);

/// 上一次按配置刷云服务页用的参数：那一页重建之后，新控件要按它填一遍。
struct CloudConfig {
    config: Config,

    key_present: bool,

    model_present: bool,
}

impl PreferencesWindow {
    /// `languages` 是打进包里的释义表语言，`version` / `build` 显示在「关于」页。
    pub fn new(mtm: MainThreadMarker, languages: &[Language], version: &str, build: &str) -> Self {
        let target = PreferencesTarget::new(mtm);
        let new_layout = || Layout::new(PAGE_WIDTH, PAGE_TOP);
        let page = |title: &'static str, layout: Layout| -> Page {
            (title, layout, flipped::view_of(&FlippedView::new(mtm)))
        };
        let mut pages: Vec<Page> = Vec::new();

        let mut layout = new_layout();
        let general = GeneralPage::build(&mut layout, mtm, &target, languages);
        pages.push(page("通用", layout));

        let mut layout = new_layout();
        let candidates = CandidatesPage::build(&mut layout, mtm, &target);
        pages.push(page("候选窗口", layout));

        let mut layout = new_layout();
        let shortcuts = ShortcutsPage::build(&mut layout, mtm, &target);
        pages.push(page("快捷键", layout));

        let mut layout = new_layout();
        let phrases = PhrasesPage::build(&mut layout, mtm, &target);
        pages.push(page("自定义短语", layout));

        let mut layout = new_layout();
        let fuzzy = FuzzyPage::build(&mut layout, mtm, &target);
        pages.push(page("模糊音", layout));

        let mut layout = new_layout();
        let dictionaries = DictionariesPage::build(&mut layout, mtm, &target);
        pages.push(page("词库", layout));

        let mut layout = new_layout();
        let cloud_shape = dev_shape();
        let cloud = CloudPage::build(&mut layout, mtm, &target, cloud_shape);
        let cloud_tab = pages.len();
        pages.push(page("云服务", layout));

        let mut layout = new_layout();
        let advanced = AdvancedPage::build(&mut layout, mtm, &target);
        pages.push(page("高级", layout));

        let mut layout = new_layout();
        let usage = UsagePage::build(&mut layout, mtm);
        pages.push(page("统计", layout));

        let mut layout = new_layout();
        let about = build_about(&mut layout, mtm, &target, version, build);
        pages.push(page("关于", layout));

        // 打印纸的尺寸：先按最大的一页量出标签栏占掉多少，窗口起手按第一页的高度（见 fit_window_to_page）
        let tallest = pages
            .iter()
            .map(|(_, layout, _)| layout.height() + PAGE_TOP)
            .fold(PAGE_TOP, f64::max);
        let page_height = tallest.min(max_page_height(mtm));
        let probe = NSRect::new(NSPoint::ZERO, NSSize::new(PAGE_WIDTH, page_height));
        let tabs = NSTabView::initWithFrame(mtm.alloc(), probe);
        let inner = tabs.contentRect();
        let chrome_height = page_height - inner.size.height;
        let content_size = NSSize::new(
            PAGE_WIDTH + (PAGE_WIDTH - inner.size.width) + 2.0 * TAB_MARGIN,
            page_height + chrome_height + 2.0 * TAB_MARGIN + STATUS_HEIGHT,
        );
        tabs.setFrame(NSRect::new(
            NSPoint::new(TAB_MARGIN, STATUS_GAP + STATUS_HEIGHT),
            NSSize::new(
                content_size.width - 2.0 * TAB_MARGIN,
                page_height + chrome_height,
            ),
        ));
        for (title, layout, view) in pages.into_iter() {
            // 每一页按自己排出来的高度：窗口跟着当前这一页伸缩（`fit_window_to_page`）
            // 页高 = 内容高度 + 页底留白；窗口就按它伸缩，矮页不撑高
            let own_height = (layout.height() + PAGE_TOP).min(max_page_height(mtm));
            view.setFrame(NSRect::new(
                NSPoint::ZERO,
                NSSize::new(PAGE_WIDTH, own_height),
            ));
            layout.finish(&view, own_height);
            // SAFETY: identifier 允许为空；条目随 NSTabView 活着
            let item = unsafe { NSTabViewItem::initWithIdentifier(mtm.alloc(), None) };
            item.setLabel(&NSString::from_str(title));
            // 一律放进滚动视图：页比窗口高时自己滚，窗口高度又跟着当前这页变
            item.setView(Some(&scrolling(mtm, &view, own_height)));
            tabs.addTabViewItem(&item);
        }
        let content = NSView::initWithFrame(mtm.alloc(), NSRect::new(NSPoint::ZERO, content_size));
        content.addSubview(&tabs);
        let status = small_label(mtm, "");
        status.setTextColor(Some(&NSColor::systemRedColor()));
        status.setFrame(NSRect::new(
            NSPoint::new(TAB_MARGIN + PAGE_PADDING, STATUS_GAP / 2.0),
            NSSize::new(
                content_size.width - 2.0 * (TAB_MARGIN + PAGE_PADDING),
                STATUS_HEIGHT,
            ),
        ));
        content.addSubview(&status);
        let tab_change = TabChange::new(mtm);
        // SAFETY: `TabChange` 实现了 NSTabViewDelegate（见 define_class）
        tabs.setDelegate(Some(objc2::runtime::ProtocolObject::from_ref(&*tab_change)));
        let panel = PreferencesPanel::new(mtm, NSRect::new(NSPoint::ZERO, content_size));
        panel.setTitle(&NSString::from_str("素笺偏好设置"));
        panel.setContentView(Some(&content));
        panel.center();
        fit_window_to_page(&tabs, false);

        Self {
            panel,
            general,
            candidates,
            shortcuts,
            phrases,
            fuzzy,
            dictionaries,
            cloud: RefCell::new(cloud),
            tabs: tabs.clone(),
            _tab_change: tab_change,
            cloud_tab,
            cloud_shape: Cell::new(cloud_shape),
            cloud_config: RefCell::new(None),
            page_height,
            advanced,
            usage,
            about,
            status,
            _target: target,
        }
    }

    pub fn select_phrase(&self, config: &Config, index: usize) {
        self.phrases.load(config, index);
    }
    pub fn selected_phrase(&self) -> Option<usize> {
        self.phrases.selected_row()
    }
    pub fn edit_phrase(&self, config: &Config, index: Option<usize>) {
        self.phrases.edit(config, index);
    }
    pub fn close_phrase_editor(&self) {
        self.phrases.close_editor();
    }
    pub fn set_phrase_error(&self, error: &str) {
        self.phrases.set_error(error);
    }
    pub fn phrase_draft(
        &self,
        config: &Config,
    ) -> Result<(Option<usize>, qingjian_core::CustomPhrase), String> {
        Ok((self.phrases.selected(config)?, self.phrases.draft()))
    }

    /// 打开（或带到最前）。
    pub fn show(&self) {
        // 云功能是后台线程在跑，打开时先照它的现状刷一遍；设备列表与别处改过的开关问一次服务器
        qingjian_cloud_mac::refresh_account();
        self.sync_cloud_status();
        self.panel.present();
    }

    /// 打开窗口并切到「云服务」页（菜单里的「云服务设置…」）。
    pub fn show_cloud(&self) {
        qingjian_cloud_mac::refresh_account();
        self.sync_cloud_status();
        self.panel.present();
        self.tabs.selectTabViewItemAtIndex(self.cloud_tab as _);
    }

    /// 按配置刷新所有控件。`key_present` 是密钥已经有了（环境或配置里）；密钥框永远不回显值。
    /// `notice` 是配置文件的问题（解析失败或被忽略的条目），没有问题时为 `None`。
    pub fn sync(
        &self,
        config: &Config,
        key_present: bool,
        notice: Option<&str>,
        dictionaries: &[DictionaryInfo],
        update: &UpdateStatus,
    ) {
        self.dictionaries.rebuild(dictionaries);
        self.about.sync(config, update);
        self.general.sync(config);
        self.candidates.sync(config);
        self.shortcuts.sync(config);
        self.phrases.sync(config);
        self.fuzzy.sync(config);
        self.sync_cloud_status();
        let model_present = crate::app::paths::p2c_model_path().is_some()
            || crate::app::paths::model_path().is_some();
        *self.cloud_config.borrow_mut() = Some(CloudConfig {
            config: config.clone(),
            key_present,
            model_present,
        });
        // 换了服务（素笺云 ↔ 自定义接口）要增减「自定义接口」那张卡
        let current = self.cloud_shape.get();
        let shape = CloudShape {
            custom: config.predict.provider == PredictProvider::Custom,
            ..current
        };
        if shape != self.cloud_shape.get() {
            self.rebuild_cloud_page(shape);
        } else {
            self.cloud.borrow().sync(config, key_present, model_present);
        }
        self.advanced.sync(config);
        // `notice` 里已经写好了「沿用上一份」/「已忽略」的措辞，这里原样显示
        let status = notice.unwrap_or_default();
        self.status.setTextColor(Some(&NSColor::systemRedColor()));
        self.status.setStringValue(&NSString::from_str(status));
    }

    /// 开发自检（只在 debug 构建里有）：把设置窗口（停在「云服务」页）截成 PNG 再退出。
    /// 输入法进程不方便手动点菜单，`QJ_SETTINGS_SHOT` 指了路径就启动后自动跑一遍（见 `host::init`）。
    #[cfg(debug_assertions)]
    pub fn dump_cloud_page(&self, path: &str) {
        self.show_cloud();
        // 想截别的页就先 `QJ_SETTINGS_TAB=<下标>`（自检窗口高度跟不跟着页面走）
        if let Ok(index) = std::env::var("QJ_SETTINGS_TAB")
            && let Ok(index) = index.parse::<isize>()
        {
            self.tabs.selectTabViewItemAtIndex(index);
        }
        let number = self.panel.windowNumber();
        let path = path.to_owned();
        // 起手的形状按假状态算（见 `dev_shape`），窗口装得下整页，只截一张就够
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(1200));
            shoot(number, &path);
            std::process::exit(0);
        });
    }

    /// 「云服务」页「高级」的展开三角：形状里翻一下，整页按新形状重建。
    pub fn toggle_cloud_advanced(&self) {
        let shape = CloudShape {
            advanced: !self.cloud_shape.get().advanced,
            ..self.cloud_shape.get()
        };
        self.rebuild_cloud_page(shape);
    }

    /// 云服务页的形状变了（开通与否、设备台数、展开高级、换服务、有没有话说）：
    /// 整个重建这一页，页面高度永远等于内容高度，不留写死高度的空位。
    fn rebuild_cloud_page(&self, shape: CloudShape) {
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };
        self.cloud_shape.set(shape);
        let mut layout = Layout::new(PAGE_WIDTH, PAGE_TOP);
        let cloud = CloudPage::build(&mut layout, mtm, &self._target, shape);
        let view = flipped::view_of(&FlippedView::new(mtm));
        let own_height = (layout.height() + PAGE_TOP).max(self.page_height);
        view.setFrame(NSRect::new(
            NSPoint::ZERO,
            NSSize::new(PAGE_WIDTH, own_height),
        ));
        layout.finish(&view, own_height);
        // 高度会随形状变，一律放进滚动视图；比可视区矮时不会出滚动条
        let item = self.tabs.tabViewItemAtIndex(self.cloud_tab as isize);
        item.setView(Some(&scrolling(mtm, &view, own_height)));
        // 新控件按当前状态与配置填一遍
        if let Some(config) = self.cloud_config.borrow().as_ref() {
            cloud.sync(&config.config, config.key_present, config.model_present);
        }
        cloud.sync_status(&cloud_status());
        *self.cloud.borrow_mut() = cloud;
        // 这一页自己变高了（多出设备行、同步与数据）或变矮了，窗口跟着走
        fit_window_to_page(&self.tabs, true);
    }

    /// 只刷「云服务」页的状态（云功能是独立的线程在跑，不等 config 变化）：
    /// 设备台数或开通状态变了就整页重建，其余情况只改文字与勾选。
    pub fn sync_cloud_status(&self) {
        let status = cloud_status();
        let current = self.cloud_shape.get();
        let shape = shape_of(&status, current.custom, current.advanced);
        if shape != self.cloud_shape.get() {
            self.rebuild_cloud_page(shape);
        } else {
            self.cloud.borrow().sync_status(&status);
        }
    }

    /// 检查更新的状态变了（查完了、查到新版），只刷「关于」页。
    pub fn sync_update(&self, config: &Config, update: &UpdateStatus) {
        self.about.sync(config, update);
    }

    /// 刷新「统计」页。打开窗口时调（数字随时在变，不跟配置一起同步）。
    pub fn sync_usage(
        &self,
        summary: &UsageSummary,
        vocabulary: &VocabularySummary,
        language: Option<Language>,
    ) {
        self.usage.show(
            summary,
            vocabulary,
            language.map_or("学习语言已关", language_label),
        );
    }

    /// 底部状态行临时显示一句提示（不是错误，灰字）；下次 `sync` 会被配置状态覆盖。
    pub fn set_status(&self, text: &str) {
        self.status
            .setTextColor(Some(&NSColor::secondaryLabelColor()));
        self.status.setStringValue(&NSString::from_str(text));
    }
}

/// 一页最高能多高：主屏可用高度减去标题栏、标签栏、状态行与留白。取不到屏幕就不封顶。
fn max_page_height(mtm: MainThreadMarker) -> f64 {
    NSScreen::mainScreen(mtm).map_or(f64::MAX, |screen| {
        screen.visibleFrame().size.height - SCREEN_MARGIN - 2.0 * TAB_MARGIN - STATUS_HEIGHT
    })
}

/// 一页装进滚动视图：每页都这么包（窗口高度跟着当前这页变，装不下时这一页自己滚），
/// 开始时停在页顶。
fn scrolling(mtm: MainThreadMarker, page: &NSView, page_height: f64) -> Retained<NSScrollView> {
    let scroll = NSScrollView::initWithFrame(
        mtm.alloc(),
        NSRect::new(NSPoint::ZERO, NSSize::new(PAGE_WIDTH, page_height)),
    );
    scroll.setHasVerticalScroller(true);
    scroll.setAutohidesScrollers(true);
    scroll.setDrawsBackground(false);
    // 宽度跟可视区域走（经典滚动条会占掉一条），免得内容比可视区宽、带出横向滚动
    let clip: Retained<NSClipView> = scroll.contentView();
    page.setFrameSize(NSSize::new(clip.bounds().size.width, page_height));
    scroll.setDocumentView(Some(page));
    // 页面视图没有翻转坐标，页顶在 y 最大处
    clip.scrollToPoint(NSPoint::ZERO);
    scroll.reflectScrolledClipView(&clip);
    scroll
}

/// 窗口跟着当前这一页排出来的高度伸缩：切页签由 [`TabChange`] 调，页面自己变高
/// （云服务页重建）时窗口这边调。顶边不动、带动画；到屏幕可视高度的上限就封顶，
/// 超出的部分这一页自己滚。
fn fit_window_to_page(tab_view: &NSTabView, animate: bool) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let (Some(item), Some(window)) = (tab_view.selectedTabViewItem(), tab_view.window()) else {
        return;
    };
    let Some(page) = item
        .view(mtm)
        .and_then(|view| view.downcast::<NSScrollView>().ok())
        .and_then(|scroll| scroll.documentView())
    else {
        return;
    };
    let height = page.frame().size.height.min(max_page_height(mtm));
    // 标签栏与边框占掉多少：拿现在的 frame 与内容区一减就是
    let frame = tab_view.frame();
    let content = tab_view.contentRect();
    let tabs_size = NSSize::new(
        PAGE_WIDTH + (frame.size.width - content.size.width),
        height + (frame.size.height - content.size.height),
    );
    // 窗口的高度要把标题栏算进去：`setFrame` 收的是含标题栏的 frame
    let content_size = NSSize::new(
        tabs_size.width + 2.0 * TAB_MARGIN,
        tabs_size.height + TAB_MARGIN + STATUS_GAP + STATUS_HEIGHT,
    );
    let window_size = window
        .frameRectForContentRect(NSRect::new(NSPoint::ZERO, content_size))
        .size;
    // 顶边不动：往下长（或往上收）
    let current = window.frame();
    let origin = NSPoint::new(
        current.origin.x,
        current.origin.y + current.size.height - window_size.height,
    );
    window.setFrame_display_animate(NSRect::new(origin, window_size), true, animate);
    let tabs_rect = NSRect::new(
        NSPoint::new(TAB_MARGIN, STATUS_GAP + STATUS_HEIGHT),
        tabs_size,
    );
    // 视图侧没有动画版的 setFrame：一步摆好，窗口那边带动画长（或收）过去
    tab_view.setFrame(tabs_rect);
}

define_class!(
    // SAFETY: NSObject 没有子类化要求；没有实现 Drop。
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    /// 换页签：窗口高度跟着新页面走。
    struct TabChange;

    impl TabChange {
        #[unsafe(method(tabView:didSelectTabViewItem:))]
        fn did_select(&self, view: Option<&NSTabView>, _item: Option<&NSTabViewItem>) {
            if let Some(view) = view {
                fit_window_to_page(view, true);
            }
        }
    }

    unsafe impl NSObjectProtocol for TabChange {}
    unsafe impl NSTabViewDelegate for TabChange {}
);

impl TabChange {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}
