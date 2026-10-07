//! 「云服务」页：状态（开通 / 加入 / 解绑）、五项云功能开关、同步（暂停 / 立即同步）、
//! 数据（清空云端输入记录）、高级（本地整句模型、云联想与自定义接口）。
//! 每块一张分组卡（[`Card`]）：一行左边是名称与说明小字，右边是控件。
//! 功能开关切的是服务器上的许可（素笺云），名字与说明与 iOS 的云功能清单一字不差。

use objc2::rc::Retained;
use objc2::{MainThreadMarker, sel};
use objc2_app_kit::{
    NSButton, NSColor, NSControlStateValueOn, NSPopUpButton, NSSecureTextField, NSSwitch,
    NSTextField, NSView,
};
use objc2_foundation::NSString;
use qingjian_cloud_mac::CloudStatus;
use qingjian_platform::Config;
use qingjian_predict::PredictProvider;

use crate::preferences::card::{Card, RowSlot, SWITCH_WIDTH, view};
use crate::preferences::controls::small_label;
use crate::preferences::controls::{
    button, button_width, danger_button, disclosure, popup, secure_field, select, set_switch,
    switch, text_field,
};
use crate::preferences::device_list::{self, DeviceList};
use crate::preferences::layout::{Layout, PAGE_PADDING, ROW_GAP};
use crate::preferences::setting::Setting;
use crate::preferences::target::PreferencesTarget;

/// 云端词槽位弹出菜单的上限（配置文件里可以填更大，菜单只列到这）。
const MAX_CLOUD_SLOTS: usize = 4;

/// 组标题那一行的高度（小号灰字，在卡片框外上方）。
const TITLE_H: f64 = 15.0;

/// 组标题与卡片之间的距离。
const TITLE_GAP: f64 = 5.0;

/// 「高级」的三角按钮与标题之间的距离。
const TITLE_INDENT: f64 = 18.0;

/// 名称那一行留两行高：状态行与同步行的文字会长到两三行，一行装不下。
const NAME_TALL: f64 = 34.0;

/// 出错那一行的高。
const HINT_H: f64 = 16.0;

/// 弹出菜单（服务、云端词位置）的宽。
const POPUP_W: f64 = 150.0;

/// 接口地址、模型、密钥输入框的宽。
const FIELD_W: f64 = 220.0;

/// 没开通时状态卡下面那行：怎么用匹配码把这台 Mac 加进去。
const JOIN_HINT: &str = "手机上已经开通的话走「输入匹配码加入」：手机上「我 → 素笺云服务 → 添加一台设备」出码，这里输码，手机上点允许。";

/// 在页面上开一张分组卡：组标题在框外上方，卡片本体交给布局器摆。
fn card(layout: &mut Layout, mtm: MainThreadMarker, title: &str) -> Card {
    let (label, card) = Card::new(mtm, layout.inner_width(), title);
    layout.place(&label, PAGE_PADDING, layout.inner_width(), TITLE_H);
    layout.next_row(TITLE_H + TITLE_GAP);
    card
}

pub struct CloudPage {
    /// 状态行的名称：未开通素笺云 / 已开通 · 连接中… 。
    state: Retained<NSTextField>,

    /// 状态卡下面那行：出错时是原因（红字），没开通时是怎么加入；都没有就藏起来。
    hint: Retained<NSTextField>,

    /// 上面那行在页面里占的高度（含它后面的行距）；藏起来时页面要减掉。
    hint_height: f64,

    /// 状态行右边的三颗按钮：没开通时显示前两颗，开通后只剩第三颗。
    create: Retained<NSButton>,

    join: Retained<NSButton>,

    unbind: Retained<NSButton>,

    /// 三颗按钮的宽（按标题算，重排时要）。
    create_w: f64,

    join_w: f64,

    unbind_w: f64,

    /// 状态行的位置：两套按钮轮流上场，按状态重新右对齐。
    status_slot: RowSlot,

    /// 状态卡的高度（换算行的位置时要用）。
    status_height: f64,

    /// 同一空间里的设备列表。
    devices: DeviceList,

    /// 五项云功能开关。
    memory: Retained<NSSwitch>,

    input_log: Retained<NSSwitch>,

    sync_switch: Retained<NSSwitch>,

    clipboard: Retained<NSSwitch>,

    llm: Retained<NSSwitch>,

    /// 同步行的名称：学习数据：刚刚同步 / 同步失败… 。
    sync_state: Retained<NSTextField>,

    pause: Retained<NSButton>,

    sync_now: Retained<NSButton>,

    clear_log: Retained<NSButton>,

    /// 高级：展开三角，以及它管着的两张卡（收起时整张藏起来）。
    advanced_toggle: Retained<NSButton>,

    advanced_title: Retained<NSTextField>,

    advanced_view: Retained<NSView>,

    custom_title: Retained<NSTextField>,

    custom_view: Retained<NSView>,

    /// 「高级」那一块（组标题 + 卡片）在页面里占的高度；收起时页面要减掉。
    advanced_height: f64,

    /// 「自定义接口」那一张卡占的高度；走素笺云时要减掉。
    custom_height: f64,

    /// 高级：本地整句模型开关。
    local_model: Retained<NSSwitch>,

    /// 高级：云联想（走素笺云时用云端的大模型代理，自定义接口时填下面那张卡）。
    enabled: Retained<NSSwitch>,

    /// 云端词槽位数（0–4）。
    slots: Retained<NSPopUpButton>,

    /// 素笺云 / 自定义接口。
    provider: Retained<NSPopUpButton>,

    /// 自定义接口那张卡里的三行输入框。
    custom: CustomRows,

    /// 「测试连接」按钮。
    test: Retained<NSButton>,
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
    ) -> Self {
        // 状态：左边状态与设备名，右边按有没有开通换一组按钮
        let mut status_card = card(layout, mtm, "状态");
        let create = button(mtm, "开通素笺云", Setting::CloudCreateSpace, target);
        let join = button(mtm, "输入匹配码加入…", Setting::CloudJoinWithCode, target);
        let unbind = danger_button(mtm, "解绑这台 Mac…", Setting::CloudUnbind, target);
        let create_w = button_width("开通素笺云");
        let join_w = button_width("输入匹配码加入…");
        let unbind_w = button_width("解绑这台 Mac…");
        join.setToolTip(Some(&NSString::from_str(JOIN_HINT)));
        let (state, status_slot) = status_card.row_group(
            mtm,
            "未开通素笺云",
            None,
            &[(view(&create), create_w), (view(&join), join_w)],
            NAME_TALL,
        );
        // 「解绑」只在开通后出现，位置按状态重排，不挤文字列
        status_card.row_extra(&status_slot, view(&unbind), unbind_w);
        // 同一空间里的设备跟在状态行下面：本机标出来，别的设备可以在这里解绑
        let devices = DeviceList::new(mtm, status_card.inner_width(), target);
        status_card.row_full(devices.view(), device_list::HEIGHT);
        let status_height = status_card.finish(layout);
        // 出错时说明原因（红字）；没话说时整行藏起来，页面高度跟着减
        let hint = small_label(mtm, "");
        hint.setTextColor(Some(&NSColor::systemRedColor()));
        layout.place(&hint, PAGE_PADDING, layout.inner_width(), HINT_H);
        layout.next_row(HINT_H);
        let hint_height = HINT_H + ROW_GAP;

        // 功能（名字与 iOS 的功能清单一字一致）
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
        feature_card.finish(layout);

        // 同步与数据：左边是最近一次同步的状态，右边两颗按钮；下面一行是清空云端输入记录
        let mut sync_card = card(layout, mtm, "同步与数据");
        let pause = button(mtm, "暂停同步", Setting::CloudPause, target);
        let sync_now = button(mtm, "立即同步", Setting::CloudSyncNow, target);
        let sync_state = sync_card.row_tall(
            mtm,
            "学习数据：正在同步…",
            None,
            &[
                (view(&pause), button_width("暂停同步")),
                (view(&sync_now), button_width("立即同步")),
            ],
            NAME_TALL,
        );
        let clear_log = danger_button(
            mtm,
            "清空云端输入记录…",
            Setting::CloudClearInputLog,
            target,
        );
        sync_card.row_with_buttons(
            mtm,
            "",
            Some("服务器上已上传的输入记录全删，本机日志也清；学到的词与设置不受影响。"),
            &[(view(&clear_log), button_width("清空云端输入记录…"))],
        );
        sync_card.finish(layout);

        // 高级：三角在组标题左边，默认收起
        let (advanced_title, mut advanced_card) = Card::new(mtm, layout.inner_width(), "高级");
        let advanced_toggle = disclosure(mtm, sel!(toggleCloudAdvanced:), target);
        layout.place(&advanced_toggle, PAGE_PADDING, TITLE_H, TITLE_H);
        layout.place(
            &advanced_title,
            PAGE_PADDING + TITLE_INDENT,
            layout.inner_width() - TITLE_INDENT,
            TITLE_H,
        );
        layout.next_row(TITLE_H + TITLE_GAP);
        let local_model = switch(mtm, Setting::LocalModelEnabled, target);
        advanced_card.row(
            mtm,
            "本地整句模型",
            Some("随包的小模型在本机给整句候选重新排序，全程离线；停键后几十毫秒生效。关掉只用词库统计。"),
            view(&local_model),
            SWITCH_WIDTH,
        );
        let enabled = switch(mtm, Setting::CloudEnabled, target);
        advanced_card.row(
            mtm,
            "启用云联想",
            Some("开启后组句时把光标附近的几十个字发给大模型，补全整句、联想下文；密码框里绝不发送。"),
            view(&enabled),
            SWITCH_WIDTH,
        );
        let slot_titles: Vec<String> = (0..=MAX_CLOUD_SLOTS)
            .map(|n| match n {
                0 => "不要（只要整句补全）".to_owned(),
                n => format!("{n} 格"),
            })
            .collect();
        let slots = popup(mtm, &slot_titles, Setting::CloudSlots, target);
        advanced_card.row(
            mtm,
            "云端词位置",
            Some("云端词到了只补进第一页末尾这几格（比如 2 就是 8、9），前面的本地候选不动；没到就什么都不变，翻页后全是本地候选。"),
            view(&slots),
            POPUP_W,
        );
        let provider = popup(
            mtm,
            &["素笺云".to_owned(), "自定义接口".to_owned()],
            Setting::CloudProvider,
            target,
        );
        advanced_card.row(
            mtm,
            "服务",
            Some("素笺云用上面的「大模型」开关，不必填密钥。自定义接口可以接任何 OpenAI 兼容的服务，填下面那张卡。"),
            view(&provider),
            POPUP_W,
        );
        let test = button(mtm, "测试连接", Setting::TestCloud, target);
        advanced_card.row(
            mtm,
            "测试连接",
            Some("按当前的服务发一条最小请求，结果显示在窗口底部。输入法进程看不到终端里的代理变量，走不通时先查这个。"),
            view(&test),
            button_width("测试连接"),
        );
        let advanced_view = advanced_card.box_view();
        let advanced_height = TITLE_H + TITLE_GAP + advanced_card.finish(layout) + ROW_GAP;

        // 自定义接口：走素笺云时整张卡收起来（不占卡内的空位）
        let (custom_title, mut custom_card) = Card::new(mtm, layout.inner_width(), "自定义接口");
        layout.place(&custom_title, PAGE_PADDING, layout.inner_width(), TITLE_H);
        layout.next_row(TITLE_H + TITLE_GAP);
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
        let custom_view = custom_card.box_view();
        let custom_height = TITLE_H + TITLE_GAP + custom_card.finish(layout) + ROW_GAP;

        let page = Self {
            state,
            hint,
            hint_height,
            create,
            join,
            unbind,
            create_w,
            join_w,
            unbind_w,
            status_slot,
            status_height,
            devices,
            memory,
            input_log,
            sync_switch,
            clipboard,
            llm,
            sync_state,
            pause,
            sync_now,
            clear_log,
            advanced_toggle,
            advanced_title,
            advanced_view,
            custom_title,
            custom_view,
            advanced_height,
            custom_height,
            local_model,
            enabled,
            slots,
            provider,
            custom: CustomRows {
                base_url,
                model,
                api_key,
            },
            test,
        };
        page.toggle_advanced();
        page
    }

    /// 状态块与开关的勾选：读素笺云现在的状态（`qingjian_cloud_mac::status`）。
    /// 每 0.5 秒跟着素笺云的菜单刷新走（见 `PreferencesWindow::sync_cloud_status`）。
    pub fn sync_status(&self, status: &CloudStatus) {
        // 正在等另一台设备允许时，状态行说得更具体；本机设备名并进来（说明小字不占行）
        let line = if status.joining {
            "正在等另一台设备允许…（手机上会弹出一条申请）".to_owned()
        } else if status.signed_in && !status.device.is_empty() {
            format!("{} · 这台设备：{}", status.line, status.device)
        } else {
            status.line.clone()
        };
        self.state.setStringValue(&NSString::from_str(&line));
        // 出错时说明原因（红字）
        let note = status.note.as_deref().unwrap_or_default();
        self.hint.setStringValue(&NSString::from_str(note));
        self.hint.setHidden(note.is_empty());
        // 右边一组按钮：没开通是「开通」「加入」，开通后只剩「解绑」
        self.create.setHidden(status.signed_in);
        self.join.setHidden(status.signed_in);
        self.unbind.setHidden(!status.signed_in);
        // 正在等另一台设备允许时，两颗按钮都灰掉（别让人重复点）
        self.create.setEnabled(!status.joining);
        self.join.setEnabled(!status.joining);
        let mut buttons: Vec<(&NSView, f64)> = Vec::new();
        if status.signed_in {
            buttons.push((view(&self.unbind), self.unbind_w));
        } else {
            buttons.push((view(&self.create), self.create_w));
            buttons.push((view(&self.join), self.join_w));
        }
        self.status_slot.align_right(self.status_height, &buttons);
        self.devices.rebuild(&status.devices, status.signed_in);
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
        self.sync_state
            .setStringValue(&NSString::from_str(if status.sync_line.is_empty() {
                if status.signed_in {
                    "学习数据：还没有同步过"
                } else {
                    ""
                }
            } else {
                &status.sync_line
            }));
        self.pause.setTitle(&NSString::from_str(if status.paused {
            "继续同步"
        } else {
            "暂停同步"
        }));
        self.pause.setEnabled(status.signed_in);
        // 没在同步（没开学习数据同步或没开通）时「立即同步」没有意义
        self.sync_now
            .setEnabled(status.signed_in && !status.sync_line.is_empty());
        self.clear_log.setEnabled(status.signed_in);
    }

    /// `key_present` 是密钥已经有了（环境或配置里）；密钥框永远不回显值，只换占位文字。
    /// `model_present` 是包里或用户目录里有模型文件，没有就把本地模型的勾选灰掉；云联想关着时它下面的项全灰。
    pub fn sync(&self, config: &Config, key_present: bool, model_present: bool) {
        set_switch(&self.local_model, config.model.enabled && model_present);
        self.local_model.setEnabled(model_present);
        set_switch(&self.enabled, config.predict.enabled);
        let cloud = config.predict.enabled;
        let custom = config.predict.provider == PredictProvider::Custom;
        self.slots.setEnabled(cloud);
        self.provider.setEnabled(cloud);
        self.test.setEnabled(cloud);
        select(&self.slots, Some(config.predict.slots.min(MAX_CLOUD_SLOTS)));
        select(&self.provider, Some(usize::from(custom)));
        self.custom.sync(config, cloud, key_present);
        self.toggle_advanced();
    }

    /// 页面里现在藏着的几块一共多高（那行提示、收起的高级、走素笺云时的自定义接口卡）：
    /// 窗口按「内容高度 = 全长 - 这个」定页面高度，收起后下面不留空。
    pub fn hidden_height(&self) -> f64 {
        let mut hidden = 0.0;
        if self.hint.isHidden() {
            hidden += self.hint_height;
        }
        if !self.advanced_expanded() {
            hidden += self.advanced_height;
        }
        if !self.custom_shown() {
            hidden += self.custom_height;
        }
        hidden
    }

    /// 「高级」展开着没有。
    fn advanced_expanded(&self) -> bool {
        self.advanced_toggle.state() == NSControlStateValueOn
    }

    /// 现在用的是自定义接口（那张卡该露面）。
    fn custom_shown(&self) -> bool {
        self.provider.indexOfSelectedItem() == 1
    }

    /// 「高级」的展开 / 收起：收起时连「自定义接口」那张卡一起藏起来。
    /// 页面高度按展开算，收起后下面留白（切页时窗口不跳）。
    pub fn toggle_advanced(&self) {
        let expanded = self.advanced_expanded();
        let custom = self.custom_shown();
        self.advanced_title.setHidden(!expanded);
        self.advanced_view.setHidden(!expanded);
        self.custom_title.setHidden(!expanded || !custom);
        self.custom_view.setHidden(!expanded || !custom);
    }
}

impl CustomRows {
    /// 自定义接口那三行的内容与可编辑状态（整张卡显不显示由页面按「服务」这一项定）。
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
