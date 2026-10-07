//! 偏好设置窗口本体：把各页（`pages/`）装进标签视图，底部一行状态；刷新时逐页同步。

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSClipView, NSColor, NSScreen, NSScrollView, NSTabView, NSTabViewItem, NSTextField, NSView,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use qingjian_core::{Language, UsageSummary, VocabularySummary};
use qingjian_platform::Config;

use super::controls::{language_label, small_label};
use super::layout::{Layout, PAGE_PADDING, PAGE_WIDTH};
use super::pages::{
    AboutPage, AdvancedPage, CandidatesPage, CloudPage, DictionariesPage, FuzzyPage, GeneralPage,
    PhrasesPage, ShortcutsPage, UpdateStatus, UsagePage, build_about,
};
use super::panel::PreferencesPanel;
use super::target::PreferencesTarget;
use crate::host::DictionaryInfo;

/// 每页顶部留白、页面最低高度（矮页也撑到这个高度，切页时窗口不跳）。
const PAGE_TOP: f64 = 10.0;
const MIN_PAGE_HEIGHT: f64 = 200.0;

/// 标签视图四周留白、底部状态行高度。
const TAB_MARGIN: f64 = 14.0;
const STATUS_HEIGHT: f64 = 18.0;

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

    /// 「云服务」页。
    cloud: CloudPage,

    /// 标签视图；「云服务设置…」要切到云服务页。
    tabs: Retained<NSTabView>,

    /// 「云服务」页在标签里的下标。
    cloud_tab: usize,

    /// 「云服务」页的承载视图与布局记录；高度随状态变，要按内容重摆（见 [`Self::fit_cloud_page`]）。
    cloud_pane: Option<CloudPane>,

    /// 标签页区的高度（各页都按它封顶，云服务页收起后也不矮过它）。
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

/// 一页：标题、布局器、承载视图。
type Page = (&'static str, Layout, Retained<NSView>);

/// 「云服务」页的承载视图、布局记录与滚动视图；内容高度会变，重摆时三样都要。
struct CloudPane {
    view: Retained<NSView>,

    /// 布局记录；重摆要按新的总高度再算一遍 frame。
    layout: Layout,

    scroll: Retained<NSScrollView>,

    /// 一块都不藏时的高度（云服务页刚建出来时的内容高度）。
    full_height: f64,
}

impl PreferencesWindow {
    /// `languages` 是打进包里的释义表语言，`version` / `build` 显示在「关于」页。
    pub fn new(mtm: MainThreadMarker, languages: &[Language], version: &str, build: &str) -> Self {
        let target = PreferencesTarget::new(mtm);
        let new_layout = || Layout::new(PAGE_WIDTH, PAGE_TOP);
        let page = |title: &'static str, layout: Layout| -> Page {
            (
                title,
                layout,
                NSView::initWithFrame(mtm.alloc(), NSRect::ZERO),
            )
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
        let cloud = CloudPage::build(&mut layout, mtm, &target);
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

        // 标签视图：先用临时尺寸量出边框与标签栏占多少，再按最高的一页定最终尺寸
        let tallest = pages
            .iter()
            .enumerate()
            .map(|(index, (_, layout, _))| {
                // 云服务页起手收着「高级」，按露在外面的高度算，不然窗口被它撑高一大截
                let hidden = if index == cloud_tab {
                    cloud.hidden_height()
                } else {
                    0.0
                };
                layout.height() + PAGE_TOP - hidden
            })
            .fold(MIN_PAGE_HEIGHT, f64::max);
        // 设置项多了以后最高的一页会超出小屏幕，窗口底部（状态行）掉到程序坞后面：窗口封顶，超高的页放进滚动视图
        let page_height = tallest.min(max_page_height(mtm)).max(MIN_PAGE_HEIGHT);
        let probe = NSRect::new(NSPoint::ZERO, NSSize::new(PAGE_WIDTH, page_height));
        let tabs = NSTabView::initWithFrame(mtm.alloc(), probe);
        let inner = tabs.contentRect();
        let chrome_width = PAGE_WIDTH - inner.size.width;
        let chrome_height = page_height - inner.size.height;
        let tabs_size = NSSize::new(PAGE_WIDTH + chrome_width, page_height + chrome_height);
        let content_size = NSSize::new(
            tabs_size.width + 2.0 * TAB_MARGIN,
            tabs_size.height + 2.0 * TAB_MARGIN + STATUS_HEIGHT,
        );
        tabs.setFrame(NSRect::new(
            NSPoint::new(TAB_MARGIN, TAB_MARGIN + STATUS_HEIGHT),
            tabs_size,
        ));
        let mut cloud_pane = None;
        for (index, (title, layout, view)) in pages.into_iter().enumerate() {
            let own_height = (layout.height() + PAGE_TOP).max(page_height);
            view.setFrame(NSRect::new(
                NSPoint::ZERO,
                NSSize::new(PAGE_WIDTH, own_height),
            ));
            layout.finish(&view, own_height);
            // SAFETY: identifier 允许为空；条目随 NSTabView 活着
            let item = unsafe { NSTabViewItem::initWithIdentifier(mtm.alloc(), None) };
            item.setLabel(&NSString::from_str(title));
            // 云服务页的高度会随状态变（收起高级、换服务），一律放进滚动视图，收起后自己变短
            let scroll =
                (index == cloud_tab).then(|| scrolling(mtm, &view, page_height, own_height));
            let scroll = scroll.or_else(|| {
                (own_height > page_height).then(|| scrolling(mtm, &view, page_height, own_height))
            });
            match &scroll {
                Some(scroll) => item.setView(Some(scroll)),
                None => item.setView(Some(&view)),
            }
            if let Some(scroll) = scroll
                && index == cloud_tab
            {
                cloud_pane = Some(CloudPane {
                    view: view.clone(),
                    layout,
                    scroll,
                    full_height: own_height,
                });
            }
            tabs.addTabViewItem(&item);
        }
        let content = NSView::initWithFrame(mtm.alloc(), NSRect::new(NSPoint::ZERO, content_size));
        content.addSubview(&tabs);
        let status = small_label(mtm, "");
        status.setTextColor(Some(&NSColor::systemRedColor()));
        status.setFrame(NSRect::new(
            NSPoint::new(TAB_MARGIN + PAGE_PADDING, TAB_MARGIN / 2.0),
            NSSize::new(
                content_size.width - 2.0 * (TAB_MARGIN + PAGE_PADDING),
                STATUS_HEIGHT,
            ),
        ));
        content.addSubview(&status);
        let panel = PreferencesPanel::new(mtm, NSRect::new(NSPoint::ZERO, content_size));
        panel.setTitle(&NSString::from_str("素笺偏好设置"));
        panel.setContentView(Some(&content));
        panel.center();

        Self {
            panel,
            general,
            candidates,
            shortcuts,
            phrases,
            fuzzy,
            dictionaries,
            cloud,
            tabs: tabs.clone(),
            cloud_tab,
            cloud_pane,
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
        self.cloud.sync(
            config,
            key_present,
            crate::app::paths::p2c_model_path().is_some()
                || crate::app::paths::model_path().is_some(),
        );
        self.fit_cloud_page();
        self.advanced.sync(config);
        // `notice` 里已经写好了「沿用上一份」/「已忽略」的措辞，这里原样显示
        let status = notice.unwrap_or_default();
        self.status.setTextColor(Some(&NSColor::systemRedColor()));
        self.status.setStringValue(&NSString::from_str(status));
    }

    /// 「云服务」页「高级」的展开三角：展开 / 收起那一组，页面高度跟着变。
    pub fn toggle_cloud_advanced(&self) {
        self.cloud.toggle_advanced();
        self.fit_cloud_page();
    }

    /// 云服务页按现在露在外面的内容重定高度：收起高级、换成素笺云时页面要跟着变短，
    /// 不然下面空一截，滚动条却在（内容比窗口矮就不该能滚）。
    fn fit_cloud_page(&self) {
        let Some(pane) = &self.cloud_pane else {
            return;
        };
        let height = (pane.full_height - self.cloud.hidden_height()).max(self.page_height);
        pane.view
            .setFrameSize(NSSize::new(pane.view.frame().size.width, height));
        // 坐标原点在左下，总高度一变每个控件的 y 都要重算
        pane.layout.finish(&pane.view, height);
        let clip: Retained<NSClipView> = pane.scroll.contentView();
        let top = (height - clip.bounds().size.height).max(0.0);
        if clip.bounds().origin.y > top {
            clip.scrollToPoint(NSPoint::new(clip.bounds().origin.x, top));
        }
        pane.scroll.reflectScrolledClipView(&clip);
    }

    /// 只刷「云服务」页的状态块与开关（云功能是独立的线程在跑，不等 config 变化）。
    pub fn sync_cloud_status(&self) {
        self.cloud.sync_status(&qingjian_cloud_mac::status());
        self.fit_cloud_page();
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

/// 把比窗口高的一页放进滚动视图，开始时停在页顶。
fn scrolling(
    mtm: MainThreadMarker,
    page: &NSView,
    visible_height: f64,
    page_height: f64,
) -> Retained<NSScrollView> {
    let scroll = NSScrollView::initWithFrame(
        mtm.alloc(),
        NSRect::new(NSPoint::ZERO, NSSize::new(PAGE_WIDTH, visible_height)),
    );
    scroll.setHasVerticalScroller(true);
    scroll.setAutohidesScrollers(true);
    scroll.setDrawsBackground(false);
    // 宽度跟可视区域走（经典滚动条会占掉一条），免得内容比可视区宽、带出横向滚动
    let clip: Retained<NSClipView> = scroll.contentView();
    page.setFrameSize(NSSize::new(clip.bounds().size.width, page_height));
    scroll.setDocumentView(Some(page));
    // 页面视图没有翻转坐标，页顶在 y 最大处
    clip.scrollToPoint(NSPoint::new(0.0, page_height - visible_height));
    scroll.reflectScrolledClipView(&clip);
    scroll
}
