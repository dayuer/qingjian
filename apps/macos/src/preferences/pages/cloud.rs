//! 「云服务」页：本地整句模型开关；云联想开关、云端词格数、走素笺云还是自定义接口（后者才有地址 / 模型 / 密钥）、测试连接。

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSButton, NSPopUpButton, NSSecureTextField, NSTextField};
use objc2_foundation::NSString;
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
    /// 本地整句模型开关。
    local_model: Retained<NSButton>,

    /// 云联想开关。
    enabled: Retained<NSButton>,

    /// 云端词槽位数（0–4）。
    slots: Retained<NSPopUpButton>,

    /// 素笺云/ 自定义接口。
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
            "开启后组句时把光标附近的几十个字发给大模型，补全整句、联想下文；密码框里绝不发送。菜单栏图标旁会带一个云朵。",
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
            "素笺云服务用菜单栏「中☁ → 素笺云 ›」里的登录账号：先登录并打开「大模型（云联想）」，这里不用填。自定义接口可以接任何 OpenAI 兼容的服务。",
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
