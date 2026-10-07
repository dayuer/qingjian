//! 「关于」页：版本与构建、检查更新、许可证、随包数据的来源与署名、隐私说明与反馈方式。
//!
//! 文案集中在这里的常量里，改措辞不用碰布局代码。第三方数据的许可证要求署名在分发物里可见，这一页就是放它的地方。

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSButton, NSFont, NSPopUpButton, NSScrollView, NSTextField, NSTextView};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use qingjian_platform::{Config, UpdateChannel};

use crate::preferences::controls::{
    GROUP_GAP, button, checkbox, paragraph, row_checkbox, row_popup, select, set_checked,
    small_label,
};
use crate::preferences::layout::{Layout, PAGE_PADDING, ROW_HEIGHT};
use crate::preferences::setting::Setting;
use crate::preferences::target::PreferencesTarget;

/// 许可证、数据来源、隐私说明那一块的高（约七行，多的自己滚）。
const TEXT_BOX_H: f64 = 110.0;

/// 许可说明，与仓库根目录 `LICENSE` 一致。
pub const LICENSE_NOTE: &str = "基于开源的青简输入法（GPL-3.0）。源码：https://github.com/dayuer/qingjian\n自由软件，GPL-3.0-or-later 许可证：可以自由使用、修改与再分发，修改后分发须同样开源。官方渠道免费。";

/// 随包数据的来源与许可证。改数据来源时同步改这里和 `apps/macos/scripts/bundle.sh` 里 `pack` 的署名。
pub const ATTRIBUTIONS: &[(&str, &str)] = &[
    (
        "词库",
        "通用规范汉字表；现代汉语常用词表（liuxilu 校对版）；THUOCL（清华大学自然语言处理实验室，MIT）；读音取自 Unihan（Unicode License v3）。",
    ),
    (
        "语言模型",
        "中文维基百科（CC BY-SA 4.0）与 LCCC（清华大学 CoAI，MIT）语料统计。",
    ),
    ("释义表", "由大语言模型（DeepSeek）生成，素笺自建。"),
    ("emoji", "Unicode CLDR annotations（Unicode License v3）。"),
    (
        "英文词表",
        "ESDB / SCOWL（© Kevin Atkinson，按其许可保留版权声明）；CSpell 词典（MIT）。",
    ),
    (
        "词汇等级",
        "The CEFR-J Wordlist Version 1.5（Yukio Tono，Tokyo University of Foreign Studies，cefr-j.org）；Octanove Vocabulary Profile C1/C2（CC BY-SA 4.0）；JLPT 词表（tanos.co.uk，CC BY；经 elzup/jlpt-word-list 整理，MIT）。",
    ),
    (
        "五笔码表",
        "86 五笔极点码表（sxjudya/rime-wubi86-jidian，Apache-2.0）；编码来自上游，词频由素笺词库按词面回填。",
    ),
];

/// 源码与问题反馈。
pub const REPOSITORY_URL: &str = "https://github.com/dayuer/qingjian";

/// 隐私说明。
pub const PRIVACY_NOTE: &str = "素笺不上传任何数据。开着「自动检查更新」时每天向 pinyin.synon.ai 读一次版本列表，请求不带任何标识，上面可以关。开着云联想或翻译时，光标附近的文字与拼音会发给你在「云服务」页填的 AI 服务商（缺省 DeepSeek）的服务器，不经过作者。「高级」页的输入日志只写在这台电脑的数据目录里，可以关掉或清空。";

/// 反馈方式。
pub const FEEDBACK_NOTE: &str = "遇到问题点「打包日志到桌面」，把生成的 zip 发给作者即可（含日志与配置文件，不含密钥），再附上「复制诊断信息」的内容。缺省日志不含你敲的内容；排查排序问题时作者可能请你在「高级」页临时打开详细日志。";

/// 检查更新的说明。
pub const UPDATE_NOTE: &str = "每天向 pinyin.synon.ai 读一次版本列表，有新版就在后台下好安装包（校验过 sha256），点「下载新版」或菜单里的「有新版本」打开安装程序。";

/// 「关于」页上检查更新那一行要显示的状态。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UpdateStatus {
    /// 查到的新版本号。
    pub available: Option<String>,

    /// 正在查。
    pub checking: bool,

    /// 查过至少一次。
    pub checked: bool,

    /// 本地开发包，不检查。
    pub dev_build: bool,
}

impl UpdateStatus {
    fn text(&self) -> String {
        match (&self.available, self.checking) {
            (Some(version), _) => format!("有新版本 {version}"),
            (None, true) => "正在检查…".to_owned(),
            (None, false) if self.dev_build => "本地开发版，不检查更新".to_owned(),
            (None, false) if self.checked => "已是最新版本".to_owned(),
            (None, false) => String::new(),
        }
    }
}

pub struct AboutPage {
    /// 自动检查更新。
    check: Retained<NSButton>,

    /// 更新渠道。
    channel: Retained<NSPopUpButton>,

    /// 「已是最新版本」/「有新版本 x.y.z」。
    status: Retained<NSTextField>,

    /// 「下载新版」，有新版才显示。
    download: Retained<NSButton>,
}

impl AboutPage {
    pub fn sync(&self, config: &Config, update: &UpdateStatus) {
        set_checked(&self.check, config.update.check);
        select(
            &self.channel,
            UpdateChannel::ALL
                .iter()
                .position(|channel| *channel == config.update.channel),
        );
        self.status
            .setStringValue(&NSString::from_str(&update.text()));
        self.download.setHidden(update.available.is_none());
    }
}

/// 正文的那一小块：只读、可选中（网址能复制），固定高度，装不下的自己滚。
fn text_box(layout: &mut Layout, mtm: MainThreadMarker, texts: &[String]) {
    let width = layout.inner_width();
    let text = NSTextView::initWithFrame(
        mtm.alloc(),
        NSRect::new(NSPoint::ZERO, NSSize::new(width, TEXT_BOX_H)),
    );
    text.setRichText(false);
    text.setEditable(false);
    text.setSelectable(true);
    text.setVerticallyResizable(true);
    text.setHorizontallyResizable(false);
    text.setFont(Some(&NSFont::systemFontOfSize(11.0)));
    text.setString(&NSString::from_str(&texts.join("\n\n")));
    let scroll = NSScrollView::initWithFrame(
        mtm.alloc(),
        NSRect::new(NSPoint::ZERO, NSSize::new(width, TEXT_BOX_H)),
    );
    scroll.setHasVerticalScroller(true);
    scroll.setAutohidesScrollers(true);
    scroll.setDocumentView(Some(&text));
    layout.place(&scroll, PAGE_PADDING, width, TEXT_BOX_H);
    layout.next_row(TEXT_BOX_H);
}

/// 把「关于」页的控件摆进 `layout`。
pub fn build(
    layout: &mut Layout,
    mtm: MainThreadMarker,
    target: &PreferencesTarget,
    version: &str,
    build: &str,
) -> AboutPage {
    let title = NSTextField::labelWithString(&NSString::from_str(&format!("素笺 {version}")), mtm);
    title.setFont(Some(&NSFont::boldSystemFontOfSize(15.0)));
    layout.place(&title, PAGE_PADDING, layout.inner_width(), ROW_HEIGHT);
    layout.next_row(ROW_HEIGHT);
    let build_label = small_label(mtm, &format!("构建 {build}"));
    layout.place(
        &build_label,
        PAGE_PADDING,
        layout.inner_width(),
        ROW_HEIGHT * 0.7,
    );
    layout.next_row(ROW_HEIGHT * 0.7);
    let repository = button(mtm, "GitHub", Setting::OpenRepository, target);
    layout.place(&repository, PAGE_PADDING, 150.0, ROW_HEIGHT + 4.0);
    layout.next_row(ROW_HEIGHT + 4.0);
    layout.space(GROUP_GAP);

    let check = checkbox(mtm, "自动检查更新", Setting::UpdateCheck, target);
    row_checkbox(layout, &check);
    let channels: Vec<String> = UpdateChannel::ALL
        .iter()
        .map(|channel| channel.label().to_owned())
        .collect();
    let channel = row_popup(
        layout,
        mtm,
        "更新渠道",
        &channels,
        Setting::UpdateChannel,
        target,
    );
    let check_now = button(mtm, "立即检查", Setting::CheckUpdateNow, target);
    let download = button(mtm, "下载新版", Setting::OpenDownload, target);
    let status = small_label(mtm, "");
    layout.place(&check_now, PAGE_PADDING, 110.0, ROW_HEIGHT + 4.0);
    layout.place(&download, PAGE_PADDING + 120.0, 110.0, ROW_HEIGHT + 4.0);
    layout.place(
        &status,
        PAGE_PADDING + 240.0,
        layout.inner_width() - 240.0,
        ROW_HEIGHT,
    );
    layout.next_row(ROW_HEIGHT + 4.0);
    paragraph(layout, mtm, UPDATE_NOTE);
    layout.space(GROUP_GAP);

    // 许可证、数据来源、隐私说明与反馈方式合成一块：全文铺开会把这一页撑到窗口装不下
    let mut texts = vec![LICENSE_NOTE.to_owned()];
    texts.extend(
        ATTRIBUTIONS
            .iter()
            .map(|(name, text)| format!("{name}：{text}")),
    );
    texts.push(PRIVACY_NOTE.to_owned());
    texts.push(FEEDBACK_NOTE.to_owned());
    let heading = small_label(mtm, "许可证、数据来源与隐私");
    layout.place(
        &heading,
        PAGE_PADDING,
        layout.inner_width(),
        ROW_HEIGHT * 0.7,
    );
    layout.next_row(ROW_HEIGHT * 0.7);
    text_box(layout, mtm, &texts);
    let open = button(mtm, "打开日志目录", Setting::OpenLogDirectory, target);
    let export = button(mtm, "打包日志到桌面", Setting::ExportLogs, target);
    let copy = button(mtm, "复制诊断信息", Setting::CopyDiagnostics, target);
    layout.place(&open, PAGE_PADDING, 150.0, ROW_HEIGHT + 4.0);
    layout.place(&export, PAGE_PADDING + 160.0, 150.0, ROW_HEIGHT + 4.0);
    layout.place(&copy, PAGE_PADDING + 320.0, 150.0, ROW_HEIGHT + 4.0);
    layout.next_row(ROW_HEIGHT + 4.0);
    AboutPage {
        check,
        channel,
        status,
        download,
    }
}
