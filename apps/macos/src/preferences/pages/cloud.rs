//! 「云服务」页：状态（开通 / 加入 / 解绑）、同一空间里的其他设备、五项云功能开关、
//! 同步与数据（暂停 / 立即同步 / 清空云端输入记录）、高级（本地整句模型、云联想与自定义接口）。
//! 每块一张分组卡（[`Card`]）：一行左边名称、右边控件；说明挂在悬停提示上，不占行。
//!
//! 页面按 [`CloudShape`] 从头排一遍：没开通就没有设备与同步那两块，用素笺云就没有自定义接口那张卡，
//! 高级收起时那张卡根本不建。形状一变窗口把这一页整个重建（`PreferencesWindow::rebuild_cloud_page`），
//! 所以页面高度永远等于内容高度，不留写死高度的空位。

use objc2::rc::Retained;
use objc2::{MainThreadMarker, sel};
use objc2_app_kit::{
    NSButton, NSColor, NSLineBreakMode, NSPopUpButton, NSSecureTextField, NSSwitch, NSTextField,
};
use objc2_foundation::NSString;
use qingjian_cloud_mac::CloudStatus;
use qingjian_platform::Config;
use qingjian_predict::PredictProvider;

use crate::preferences::card::{Card, SWITCH_WIDTH, view};
use crate::preferences::controls::{
    action_danger_button, button, button_width, danger_button, disclosure, popup, secure_field,
    select, set_switch, switch, text_field,
};
use crate::preferences::layout::{Layout, PAGE_PADDING, ROW_GAP};
use crate::preferences::setting::Setting;
use crate::preferences::target::PreferencesTarget;

/// 云端词槽位弹出菜单的上限（配置文件里可以填更大，菜单只列到这）。
const MAX_CLOUD_SLOTS: usize = 4;

/// 组标题那一行的高度（小号灰字，在卡片框外上方）。
const TITLE_H: f64 = 15.0;

/// 组标题与卡片之间。
const TITLE_GAP: f64 = 6.0;

/// 一张卡片与下一组标题之间。
const CARD_GAP: f64 = 18.0;

/// 名称那一行留两行高：状态行会长到两三行。
const NAME_TALL: f64 = 34.0;

/// 弹出菜单（服务、云端词位置）的宽。
const POPUP_W: f64 = 150.0;

/// 接口地址、模型、密钥输入框的宽。
const FIELD_W: f64 = 220.0;

/// 没开通时「输入匹配码加入…」的说明，挂在这个按钮的悬停提示上。
const JOIN_HINT: &str = "手机上已经开通的话走「输入匹配码加入」：手机上「我 → 素笺云服务 → 添加一台设备」出码，这里输码，手机上点允许。";

/// 一台别的设备都没有时的说明。
const NO_OTHERS: &str = "还没有别的设备：在手机上用「添加一台设备」出码，新设备输码加入。";

/// 云服务页该长什么样。变了就把这一页整个重排一遍。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CloudShape {
    /// 开通了没有：没开通只有状态卡（开通 / 加入）。
    pub signed_in: bool,

    /// 同一空间里**别的**设备的台数（本机在状态行上）；开通后至少留一行放提示。
    pub others: usize,

    /// 用的是自定义接口（那一张卡要露面）。
    pub custom: bool,

    /// 「高级」展开着（收起时那张卡不建）。
    pub advanced: bool,

    /// 有要说的提示（加入失败、已清空等）：占一行红字，没有就不占。
    pub note: bool,
}

impl CloudShape {
    /// 起手的样子：没开通、没展开、没话说。
    pub fn closed() -> Self {
        Self {
            signed_in: false,
            others: 0,
            custom: false,
            advanced: false,
            note: false,
        }
    }
}

/// 在页面上开一张分组卡：组标题在框外上方。
fn card(layout: &mut Layout, mtm: MainThreadMarker, title: &str) -> Card {
    let (label, card) = Card::new(mtm, layout.inner_width(), title);
    layout.place(&label, PAGE_PADDING, layout.inner_width(), TITLE_H);
    layout.next_row(TITLE_H + TITLE_GAP);
    card
}

/// 一张卡片收尾：摆进页面，再留出与下一组之间的距离。
fn finish_card(layout: &mut Layout, card: Card) {
    card.finish(layout);
    layout.space(CARD_GAP - ROW_GAP);
}

pub struct CloudPage {
    /// 状态行的名称：未开通素笺云 / 已开通 · 同步中…（开通后跟上本机设备名）。
    state: Retained<NSTextField>,

    /// 有话说时那一行红字；形状里没有它就不建。
    hint: Option<Retained<NSTextField>>,

    /// 没开通时才有的两颗按钮。
    create: Option<Retained<NSButton>>,

    join: Option<Retained<NSButton>>,

    /// 别的设备那几行：名称标签与它的解绑按钮（按状态填文字与 tag）。
    others: Vec<(Retained<NSTextField>, Retained<NSButton>)>,

    /// 五项云功能开关。
    memory: Retained<NSSwitch>,

    input_log: Retained<NSSwitch>,

    sync_switch: Retained<NSSwitch>,

    clipboard: Retained<NSSwitch>,

    llm: Retained<NSSwitch>,

    /// 开通后才有的同步行、两颗按钮与清空按钮。
    sync_state: Option<Retained<NSTextField>>,

    pause: Option<Retained<NSButton>>,

    sync_now: Option<Retained<NSButton>>,

    clear_log: Option<Retained<NSButton>>,

    /// 高级展开后才有的那张卡里的控件。
    local_model: Option<Retained<NSSwitch>>,

    enabled: Option<Retained<NSSwitch>>,

    slots: Option<Retained<NSPopUpButton>>,

    provider: Option<Retained<NSPopUpButton>>,

    test: Option<Retained<NSButton>>,

    /// 自定义接口那张卡里的三行输入框。
    custom: Option<CustomRows>,
}

struct CustomRows {
    base_url: Retained<NSTextField>,

    model: Retained<NSTextField>,

    /// 密钥输入框，永远不回显已有值。
    api_key: Retained<NSSecureTextField>,
}

impl CloudPage {
    pub fn build(
        layout: &mut Layout,
        mtm: MainThreadMarker,
        target: &Retained<PreferencesTarget>,
        shape: CloudShape,
    ) -> Self {
        // ── 状态」，右边按开通与否换按钮）──
        let mut status_card = card(layout, mtm, "状态");
        let mut create = None;
        let mut join = None;
        let state = if shape.signed_in {
            let control = danger_button(mtm, "解绑这台 Mac…", Setting::CloudUnbind, target);
            status_card.row_tall(
                mtm,
                "已开通",
                None,
                &[(view(&control), button_width("解绑这台 Mac…"))],
                NAME_TALL,
            )
        } else {
            let create_button = button(mtm, "开通素笺云", Setting::CloudCreateSpace, target);
            let join_button = button(mtm, "输入匹配码加入…", Setting::CloudJoinWithCode, target);
            join_button.setToolTip(Some(&NSString::from_str(JOIN_HINT)));
            let label = status_card.row_tall(
                mtm,
                "未开通素笺云",
                None,
                &[
                    (view(&create_button), button_width("开通素笺云")),
                    (view(&join_button), button_width("输入匹配码加入…")),
                ],
                NAME_TALL,
            );
            create = Some(create_button);
            join = Some(join_button);
            label
        };

        // 同一空间里别的设备：一台一行，右边「解绑…」（本机在状态行上）
        let mut others = Vec::new();
        if shape.signed_in {
            for _ in 0..shape.others.max(1) {
                let control = action_danger_button(mtm, "解绑…", sel!(revokeDevice:), 0, target);
                let label =
                    status_card.row(mtm, NO_OTHERS, None, view(&control), button_width("解绑…"));
                // 名字长的尾部截断，不折行
                if let Some(cell) = label.cell() {
                    cell.setLineBreakMode(NSLineBreakMode::ByTruncatingTail);
                }
                label.setUsesSingleLineMode(true);
                others.push((label, control));
            }
        }
        // 有话说时在状态卡最后一行说（加入失败、已清空等）
        let hint = shape
            .note
            .then(|| status_card.row_text(mtm, "", Some(&NSColor::systemRedColor())));
        finish_card(layout, status_card);

        // ── 功能（名字与 iOS 的功能清单一字一致）──
        let mut feature_card = card(layout, mtm, "功能");
        let memory = switch(mtm, Setting::CloudMemory, target);
        feature_card.row(
            mtm,
            "云端记忆（把记下的素材整理成卡）",
            Some("记下的素材先在本机抹去姓名、电话、地址等再上传，交给云端整理成卡；关掉会同时删除服务器上的素材。"),
            view(&memory),
            SWITCH_WIDTH,
        );
        let input_log = switch(mtm, Setting::CloudInputLog, target);
        feature_card.row(
            mtm,
            "同步打字内容",
            Some("打的字上传到素笺的服务器，用来优化输入法；密码、验证码这类输入框不会记录。"),
            view(&input_log),
            SWITCH_WIDTH,
        );
        let sync_switch = switch(mtm, Setting::CloudSync, target);
        feature_card.row(
            mtm,
            "同步学习数据与设置",
            Some("学到的词、词频与设置在多台设备间保持一致；关掉只停同步，本机数据还在。"),
            view(&sync_switch),
            SWITCH_WIDTH,
        );
        let clipboard = switch(mtm, Setting::CloudClipboard, target);
        feature_card.row(
            mtm,
            "跨设备剪贴板",
            Some("本机复制的文本传到别的设备，别的设备复制的写进本机剪贴板。"),
            view(&clipboard),
            SWITCH_WIDTH,
        );
        let llm = switch(mtm, Setting::CloudLlm, target);
        feature_card.row(
            mtm,
            "大模型（润色、云联想）",
            Some("改写选中或整句，组句时联想整句与云端候选；走素笺云自己的服务器，不用填密钥。"),
            view(&llm),
            SWITCH_WIDTH,
        );
        if !shape.signed_in {
            feature_card.row_text(mtm, "开通后可用。", Some(&NSColor::secondaryLabelColor()));
        }
        finish_card(layout, feature_card);

        // ── 同步与数据：没开通时这几颗按钮都是灰的，整块不建 ──
        let (sync_state, pause, sync_now, clear_log) = if shape.signed_in {
            let mut sync_card = card(layout, mtm, "同步与数据");
            let pause_button = button(mtm, "暂停同步", Setting::CloudPause, target);
            let sync_now_button = button(mtm, "立即同步", Setting::CloudSyncNow, target);
            let label = sync_card.row_tall(
                mtm,
                "学习数据：正在同步…",
                None,
                &[
                    (view(&pause_button), button_width("暂停同步")),
                    (view(&sync_now_button), button_width("立即同步")),
                ],
                NAME_TALL,
            );
            let clear = danger_button(
                mtm,
                "清空云端输入记录…",
                Setting::CloudClearInputLog,
                target,
            );
            sync_card.row_with_buttons(
                mtm,
                "云端输入记录",
                Some("服务器上已上传的输入记录全删，本机日志也清；学到的词与设置不受影响。"),
                &[(view(&clear), button_width("清空云端输入记录…"))],
            );
            finish_card(layout, sync_card);
            (
                Some(label),
                Some(pause_button),
                Some(sync_now_button),
                Some(clear),
            )
        } else {
            (None, None, None, None)
        };

        // ── 高级：一行标题（三角就在按钮上），收起时不建那张卡 ──
        let advanced_toggle = disclosure(
            mtm,
            if shape.advanced {
                "▾ 高级"
            } else {
                "▸ 高级"
            },
            sel!(toggleCloudAdvanced:),
            target,
        );
        layout.place(
            &advanced_toggle,
            PAGE_PADDING,
            layout.inner_width(),
            TITLE_H,
        );
        layout.next_row(TITLE_H + TITLE_GAP);
        let mut local_model = None;
        let mut enabled = None;
        let mut slots = None;
        let mut provider = None;
        let mut test = None;
        let mut custom = None;
        if shape.advanced {
            let (_, mut advanced_card) = Card::new(mtm, layout.inner_width(), "高级");
            let local = switch(mtm, Setting::LocalModelEnabled, target);
            advanced_card.row(
                mtm,
                "本地整句模型",
                Some("随包的小模型在本机给整句候选重新排序，全程离线；停键后几十毫秒生效。关掉只用词库统计。"),
                view(&local),
                SWITCH_WIDTH,
            );
            let cloud_switch = switch(mtm, Setting::CloudEnabled, target);
            advanced_card.row(
                mtm,
                "启用云联想",
                Some("开启后组句时把光标附近的几十个字发给大模型，补全整句、联想下文；密码框里绝不发送。"),
                view(&cloud_switch),
                SWITCH_WIDTH,
            );
            let slot_titles: Vec<String> = (0..=MAX_CLOUD_SLOTS)
                .map(|n| match n {
                    0 => "不要（只要整句补全）".to_owned(),
                    n => format!("{n} 格"),
                })
                .collect();
            let slots_popup = popup(mtm, &slot_titles, Setting::CloudSlots, target);
            advanced_card.row(
                mtm,
                "云端词位置",
                Some("云端词到了只补进第一页末尾这几格（比如 2 就是 8、9），前面的本地候选不动；没到就什么都不变，翻页后全是本地候选。"),
                view(&slots_popup),
                POPUP_W,
            );
            let provider_popup = popup(
                mtm,
                &["素笺云".to_owned(), "自定义接口".to_owned()],
                Setting::CloudProvider,
                target,
            );
            advanced_card.row(
                mtm,
                "服务",
                Some("素笺云用上面的「大模型」开关，不必填密钥。自定义接口可以接任何 OpenAI 兼容的服务，填下面那张卡。"),
                view(&provider_popup),
                POPUP_W,
            );
            let test_button = button(mtm, "测试连接", Setting::TestCloud, target);
            advanced_card.row(
                mtm,
                "测试连接",
                Some("按当前的服务发一条最小请求，结果显示在窗口底部。输入法进程看不到终端里的代理变量，走不通时先查这个。"),
                view(&test_button),
                button_width("测试连接"),
            );
            finish_card(layout, advanced_card);
            local_model = Some(local);
            enabled = Some(cloud_switch);
            slots = Some(slots_popup);
            provider = Some(provider_popup);
            test = Some(test_button);

            // 自定义接口：走素笺云时这张卡不建
            if shape.custom {
                let mut custom_card = card(layout, mtm, "自定义接口");
                let base_url = text_field(mtm, Setting::BaseUrl, target);
                custom_card.row(mtm, "接口地址", None, view(&base_url), FIELD_W);
                let model = text_field(mtm, Setting::Model, target);
                custom_card.row(mtm, "模型", None, view(&model), FIELD_W);
                let api_key = secure_field(mtm, Setting::ApiKey, target);
                custom_card.row(mtm, "API 密钥", None, view(&api_key), FIELD_W);
                custom_card.row_note(
                    mtm,
                    "文本框按回车保存。密钥只保存在这台电脑上，不会随配置文件导出，也不显示已填的值。",
                );
                finish_card(layout, custom_card);
                custom = Some(CustomRows {
                    base_url,
                    model,
                    api_key,
                });
            }
        }

        Self {
            state,
            hint,
            create,
            join,
            others,
            memory,
            input_log,
            sync_switch,
            clipboard,
            llm,
            sync_state,
            pause,
            sync_now,
            clear_log,
            local_model,
            enabled,
            slots,
            provider,
            test,
            custom,
        }
    }

    /// 状态行、设备行、开关、同步行都按素笺云现在的状态（`qingjian_cloud_mac::status`）填。
    pub fn sync_status(&self, status: &CloudStatus) {
        // 正在等另一台设备允许时说得更具体；开通后跟上本机设备名
        let line = if status.joining {
            "正在等另一台设备允许…（手机上会弹出一条申请）".to_owned()
        } else if status.signed_in && !status.device.is_empty() {
            format!("{} · 这台设备：{}", status.line, status.device)
        } else {
            status.line.clone()
        };
        self.state.setStringValue(&NSString::from_str(&line));
        if let Some(hint) = &self.hint {
            hint.setStringValue(&NSString::from_str(
                status.note.as_deref().unwrap_or_default(),
            ));
        }
        // 别的设备：按服务器给的顺序填，多出来的行留提示（至少一行）
        let others: Vec<&qingjian_cloud_mac::CloudDevice> = status
            .devices
            .iter()
            .filter(|device| !device.current)
            .collect();
        for (index, (label, unbind)) in self.others.iter().enumerate() {
            match others.get(index) {
                Some(device) => {
                    label.setStringValue(&NSString::from_str(&format!(
                        "{} · {}",
                        device.name, device.detail
                    )));
                    unbind.setTag(device.id as isize);
                    unbind.setEnabled(true);
                }
                None => {
                    label.setStringValue(&NSString::from_str(NO_OTHERS));
                    unbind.setEnabled(false);
                }
            }
        }
        // 五项开关：没开通时整组灰掉
        for (control, name) in [
            (&self.memory, "memory"),
            (&self.input_log, "input_log"),
            (&self.sync_switch, "sync"),
            (&self.clipboard, "clipboard"),
            (&self.llm, "llm"),
        ] {
            let on = status
                .consents
                .iter()
                .find(|(n, _)| *n == name)
                .is_some_and(|(_, on)| *on);
            set_switch(control, on);
            control.setEnabled(status.signed_in);
        }
        if let Some(sync_state) = &self.sync_state {
            sync_state.setStringValue(&NSString::from_str(if status.sync_line.is_empty() {
                "学习数据：还没有同步过"
            } else {
                &status.sync_line
            }));
        }
        if let Some(pause) = &self.pause {
            pause.setTitle(&NSString::from_str(if status.paused {
                "继续同步"
            } else {
                "暂停同步"
            }));
            pause.setEnabled(status.signed_in);
        }
        if let Some(sync_now) = &self.sync_now {
            // 没在同步（没开学习数据同步）时「立即同步」没有意义
            sync_now.setEnabled(status.signed_in && !status.sync_line.is_empty());
        }
        if let Some(clear_log) = &self.clear_log {
            clear_log.setEnabled(status.signed_in);
        }
        // 正在等另一台设备允许时，两颗按钮都灰掉（别让人重复点）
        for control in [&self.create, &self.join].into_iter().flatten() {
            control.setEnabled(!status.joining);
        }
    }

    /// `key_present` 是密钥已经有了（环境或配置里）；密钥框永远不回显值，只换占位文字。
    /// `model_present` 是包里或用户目录里有模型文件，没有就把本地模型的勾选灰掉。
    pub fn sync(&self, config: &Config, key_present: bool, model_present: bool) {
        let cloud = config.predict.enabled;
        if let Some(local_model) = &self.local_model {
            set_switch(local_model, config.model.enabled && model_present);
            local_model.setEnabled(model_present);
        }
        if let Some(enabled) = &self.enabled {
            set_switch(enabled, cloud);
        }
        if let Some(slots) = &self.slots {
            slots.setEnabled(cloud);
            select(slots, Some(config.predict.slots.min(MAX_CLOUD_SLOTS)));
        }
        if let Some(provider) = &self.provider {
            let custom = config.predict.provider == PredictProvider::Custom;
            provider.setEnabled(cloud);
            select(provider, Some(usize::from(custom)));
        }
        if let Some(test) = &self.test {
            test.setEnabled(cloud);
        }
        if let Some(custom) = &self.custom {
            custom.sync(config, cloud, key_present);
        }
    }
}

impl CustomRows {
    /// 自定义接口那三行的内容与可编辑状态（整张卡显不显示由形状定）。
    fn sync(&self, config: &Config, cloud: bool, key_present: bool) {
        for field in [&*self.base_url, &*self.model, &*self.api_key] {
            field.setEnabled(cloud);
        }
        self.base_url
            .setStringValue(&NSString::from_str(&config.predict.base_url));
        self.model
            .setStringValue(&NSString::from_str(&config.predict.model));
        self.api_key.setStringValue(&NSString::from_str(""));
        let hint = if key_present {
            "已设置，输入新值可替换"
        } else {
            "未设置"
        };
        self.api_key
            .setPlaceholderString(Some(&NSString::from_str(hint)));
    }
}
