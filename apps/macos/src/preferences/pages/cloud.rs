//! 「云服务」页：状态（开通 / 加入 / 解绑）、五项云功能开关、同步（暂停 / 立即同步）、
//! 数据（清空云端输入记录）、高级（本地整句模型、云联想端点与自定义接口）。
//! 功能开关切的是服务器上的许可（素笺云），名字与说明与 iOS 的云功能清单一字不差。

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSButton, NSPopUpButton, NSSecureTextField, NSTextField};
use objc2_foundation::NSString;
use qingjian_cloud_mac::CloudStatus;
use qingjian_platform::Config;
use qingjian_predict::PredictProvider;

use crate::preferences::controls::{
    button, checkbox, note, note_label, row_checkbox, row_control, row_popup, secure_field, select,
    set_checked, text_field,
};
use crate::preferences::layout::{Layout, PAGE_PADDING, ROW_HEIGHT};
use crate::preferences::setting::Setting;
use crate::preferences::target::PreferencesTarget;

/// 云端词槽位弹出菜单的上限（配置文件里可以填更大，菜单只列到这）。
const MAX_CLOUD_SLOTS: usize = 4;

pub struct CloudPage {
    /// 状态行：未开通素笺云 / 已开通 · 同步中 / 已暂停。
    state: Retained<NSTextField>,

    /// 已开通时的设备行：这台设备的名字。
    device: Retained<NSTextField>,

    /// 没开通时才显示的两颗按钮。
    create: Retained<NSButton>,

    join: Retained<NSButton>,

    /// 已开通时才显示。
    unbind: Retained<NSButton>,

    /// 五项云功能开关。
    memory: Retained<NSButton>,

    input_log: Retained<NSButton>,

    sync_switch: Retained<NSButton>,

    clipboard: Retained<NSButton>,

    llm: Retained<NSButton>,

    /// 学习数据的同步状态（最近同步多久前 / 失败原因）。
    sync_state: Retained<NSTextField>,

    pause: Retained<NSButton>,

    sync_now: Retained<NSButton>,

    clear_log: Retained<NSButton>,

    /// 高级：本地整句模型开关。
    local_model: Retained<NSButton>,

    /// 高级：云联想（走素笺云时用云端的大模型代理，自定义接口时填下面的几行）。
    enabled: Retained<NSButton>,

    /// 云端词槽位数（0–4）。
    slots: Retained<NSPopUpButton>,

    /// 素笺云 / 自定义接口。
    provider: Retained<NSPopUpButton>,

    /// 自定义接口才有的几行：地址、模型、密钥各自的标题与输入框，加下面的说明；走素笺云时整段藏起来。
    custom: CustomRows,

    /// 「测试连接」按钮。
    test: Retained<NSButton>,
}

struct CustomRows {
    captions: [Retained<NSTextField>; 3],

    base_url: Retained<NSTextField>,

    model: Retained<NSTextField>,

    /// 密钥输入框，永远不回显已有值。
    api_key: Retained<NSSecureTextField>,

    note: Retained<NSTextField>,
}

impl CloudPage {
    pub fn build(layout: &mut Layout, mtm: MainThreadMarker, target: &PreferencesTarget) -> Self {
        // 状态
        let state = note_label(layout, mtm, "…");
        let device = note_label(layout, mtm, "");
        let create = button(mtm, "开通素笺云", Setting::CloudCreateSpace, target);
        let join = button(mtm, "输入匹配码加入…", Setting::CloudJoinWithCode, target);
        layout.place(&create, PAGE_PADDING, 190.0, ROW_HEIGHT + 4.0);
        layout.next_row(ROW_HEIGHT + 4.0);
        layout.place(&join, PAGE_PADDING, 190.0, ROW_HEIGHT + 4.0);
        layout.next_row(ROW_HEIGHT + 4.0);
        let unbind = button(mtm, "解绑这台 Mac", Setting::CloudUnbind, target);
        layout.place(&unbind, PAGE_PADDING, 190.0, ROW_HEIGHT + 4.0);
        layout.next_row(ROW_HEIGHT + 4.0);
        note(
            layout,
            mtm,
            "手机上已经开通的话走「输入匹配码加入」：手机上「我 → 素笺云服务 → 添加一台设备」出码，这里输码，手机上点允许。",
        );

        // 功能开关（名字与 iOS 的功能清单一字一致）
        let memory = checkbox(
            mtm,
            "云端记忆（把记下的素材整理成卡）",
            Setting::CloudMemory,
            target,
        );
        row_checkbox(layout, &memory);
        note(
            layout,
            mtm,
            "记下的素材先在本机抹去姓名、电话、地址等再上传，交给云端整理成卡；关掉会同时删除服务器上的素材。",
        );
        let input_log = checkbox(mtm, "同步打字内容", Setting::CloudInputLog, target);
        row_checkbox(layout, &input_log);
        note(
            layout,
            mtm,
            "打的字上传到素笺的服务器，用来优化输入法；密码、验证码这类输入框不会记录。",
        );
        let sync_switch = checkbox(mtm, "同步学习数据与设置", Setting::CloudSync, target);
        row_checkbox(layout, &sync_switch);
        let clipboard = checkbox(mtm, "跨设备剪贴板", Setting::CloudClipboard, target);
        row_checkbox(layout, &clipboard);
        note(
            layout,
            mtm,
            "本机复制的文本传到别的设备，别的设备复制的写进本机剪贴板。",
        );
        let llm = checkbox(mtm, "大模型（润色、云联想）", Setting::CloudLlm, target);
        row_checkbox(layout, &llm);
        note(
            layout,
            mtm,
            "改写选中或整句，组句时联想整句与云端候选；走素笺云自己的服务器，不用填密钥。",
        );

        // 同步
        let sync_state = note_label(layout, mtm, "");
        let pause = button(mtm, "暂停同步", Setting::CloudPause, target);
        let sync_now = button(mtm, "立即同步学习数据", Setting::CloudSyncNow, target);
        layout.place(&pause, PAGE_PADDING, 160.0, ROW_HEIGHT + 4.0);
        layout.next_row(ROW_HEIGHT + 4.0);
        layout.place(&sync_now, PAGE_PADDING, 190.0, ROW_HEIGHT + 4.0);
        layout.next_row(ROW_HEIGHT + 4.0);

        // 数据
        let clear_log = button(
            mtm,
            "清空云端输入记录…",
            Setting::CloudClearInputLog,
            target,
        );
        layout.place(&clear_log, PAGE_PADDING, 190.0, ROW_HEIGHT + 4.0);
        layout.next_row(ROW_HEIGHT + 4.0);
        note(
            layout,
            mtm,
            "服务器上已上传的输入记录全删，本机日志也清；学到的词与设置不受影响。",
        );

        // 高级
        let local_model = checkbox(mtm, "本地整句模型", Setting::LocalModelEnabled, target);
        row_checkbox(layout, &local_model);
        note(
            layout,
            mtm,
            "随包的小模型在本机给整句候选重新排序，全程离线；停键后几十毫秒生效。关掉只用词库统计。",
        );
        let enabled = checkbox(mtm, "启用云联想", Setting::CloudEnabled, target);
        row_checkbox(layout, &enabled);
        note(
            layout,
            mtm,
            "开启后组句时把光标附近的几十个字发给大模型，补全整句、联想下文；密码框里绝不发送。",
        );
        let slot_titles: Vec<String> = (0..=MAX_CLOUD_SLOTS)
            .map(|n| match n {
                0 => "不要（只要整句补全）".to_owned(),
                n => format!("{n} 格"),
            })
            .collect();
        let slots = row_popup(
            layout,
            mtm,
            "云端词位置",
            &slot_titles,
            Setting::CloudSlots,
            target,
        );
        note(
            layout,
            mtm,
            "云端词到了只补进第一页末尾这几格（比如 2 就是 8、9），前面的本地候选不动；没到就什么都不变，翻页后全是本地候选。",
        );
        let provider = row_popup(
            layout,
            mtm,
            "服务",
            &["素笺云".to_owned(), "自定义接口".to_owned()],
            Setting::CloudProvider,
            target,
        );
        note(
            layout,
            mtm,
            "素笺云用上面的「大模型」开关，不必填密钥。自定义接口可以接任何 OpenAI 兼容的服务，填下面的三行。",
        );
        let base_url = text_field(mtm, Setting::BaseUrl, target);
        let base_url_caption = row_control(layout, mtm, "接口地址", &base_url);
        let model = text_field(mtm, Setting::Model, target);
        let model_caption = row_control(layout, mtm, "模型", &model);
        let api_key = secure_field(mtm, Setting::ApiKey, target);
        let api_key_caption = row_control(layout, mtm, "API 密钥", &api_key);
        let custom_note = note_label(
            layout,
            mtm,
            "文本框按回车保存。密钥只保存在这台电脑上，不会随配置文件导出，也不显示已填的值。",
        );
        let test = button(mtm, "测试连接", Setting::TestCloud, target);
        layout.place(&test, PAGE_PADDING, 120.0, ROW_HEIGHT + 4.0);
        layout.next_row(ROW_HEIGHT + 4.0);
        note(
            layout,
            mtm,
            "按当前的服务发一条最小请求，结果显示在窗口底部。输入法进程看不到终端里的代理变量，走不通时先查这个。",
        );

        Self {
            state,
            device,
            create,
            join,
            unbind,
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
            custom: CustomRows {
                captions: [base_url_caption, model_caption, api_key_caption],
                base_url,
                model,
                api_key,
                note: custom_note,
            },
            test,
        }
    }

    /// 状态块与开关的勾选：读素笺云现在的状态（`qingjian_cloud_mac::status`）。
    /// 每 0.5 秒跟着素笺云的菜单刷新走（见 `PreferencesWindow::sync_cloud_status`）。
    pub fn sync_status(&self, status: &CloudStatus) {
        self.state.setStringValue(&NSString::from_str(&status.line));
        self.device
            .setStringValue(&NSString::from_str(&if status.signed_in {
                if status.device.is_empty() {
                    String::new()
                } else {
                    format!("这台设备：{}", status.device)
                }
            } else {
                String::new()
            }));
        self.device.setHidden(!status.signed_in);
        self.unbind.setHidden(!status.signed_in);
        self.create.setHidden(status.signed_in);
        self.join.setHidden(status.signed_in);
        // 正在等另一台设备允许时，两颗按钮都灰掉（别让人重复点）
        self.create.setEnabled(!status.joining);
        self.join.setEnabled(!status.joining);
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
            set_checked(control, on);
            control.setEnabled(status.signed_in);
        }
        self.sync_state
            .setStringValue(&NSString::from_str(if status.sync_line.is_empty() {
                if status.signed_in {
                    "还没有同步过"
                } else {
                    ""
                }
            } else {
                &status.sync_line
            }));
        self.pause.setEnabled(status.signed_in);
        self.sync_now.setEnabled(status.signed_in);
        self.clear_log.setEnabled(status.signed_in);
    }

    /// `key_present` 是密钥已经有了（环境或配置里）；密钥框永远不回显值，只换占位文字。
    /// `model_present` 是包里或用户目录里有模型文件，没有就把本地模型的勾选灰掉；云联想关着时它下面的项全灰。
    pub fn sync(&self, config: &Config, key_present: bool, model_present: bool) {
        set_checked(&self.local_model, config.model.enabled && model_present);
        self.local_model.setEnabled(model_present);
        set_checked(&self.enabled, config.predict.enabled);
        let cloud = config.predict.enabled;
        let custom = config.predict.provider == PredictProvider::Custom;
        self.slots.setEnabled(cloud);
        self.provider.setEnabled(cloud);
        self.test.setEnabled(cloud);
        select(&self.slots, Some(config.predict.slots.min(MAX_CLOUD_SLOTS)));
        select(&self.provider, Some(usize::from(custom)));
        self.custom.sync(config, cloud, custom, key_present);
    }
}

impl CustomRows {
    fn sync(&self, config: &Config, cloud: bool, custom: bool, key_present: bool) {
        for caption in &self.captions {
            caption.setHidden(!custom);
        }
        self.note.setHidden(!custom);
        for field in [&*self.base_url, &*self.model, &*self.api_key] {
            field.setHidden(!custom);
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
