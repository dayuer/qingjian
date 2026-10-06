//! 「素笺云 ›」子菜单里要先弹原生弹窗的三项：开通（出境同意）、输匹配码加入、清空云端输入记录。
//! 控件面用原生弹窗（与偏好设置同一套做法：临时切 Accessory 再激活，弹完还原），
//! 拿到结果再调 `qingjian_cloud_mac` 的对应函数。文案与 iOS 开通页（`SpaceWording`）一致。

use objc2::MainThreadMarker;
use objc2_app_kit::{
    NSAlert, NSAlertFirstButtonReturn, NSApplication, NSApplicationActivationPolicy, NSTextField,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

use qingjian_cloud_mac::{TAG_CLEAR_INPUT_LOG, TAG_CREATE_SPACE, TAG_JOIN_WITH_CODE};

/// 出境同意那一句，与 iOS `SpaceWording.consent` 一致。
const CONSENT: &str = "同意把我开启的云功能数据存在位于新加坡的服务器（腾讯云）并在那里处理，用于同步与整理记忆。可随时在「我」里关掉功能或删掉云端数据。";

/// 同意说明的正文：与 iOS `ConsentSheet.ConsentCopy` 一字一致（改一处要同步另一处，见 cloud/docs/design.md）。
fn consent_points(feature: &str) -> Option<(&'static str, [&'static str; 3])> {
    match feature {
        "memory" => Some((
            "云端记忆",
            [
                "记下的素材先在本机抹去姓名、电话、地址等，再上传到素笺的服务器（新加坡）。",
                "整理时交给 DeepSeek：数据经新加坡发往中国境内处理；DeepSeek 可能保存数据，或用于改进它的模型，具体见它官网的隐私政策。",
                "随时可以关掉；关掉会同时删除服务器上的素材。",
            ],
        )),
        "input_log" => Some((
            "同步打字内容",
            [
                "打的字会上传到素笺的服务器（新加坡），用来优化输入法。",
                "用于 AI 优化时会先脱敏再发给 DeepSeek（数据在中国境内处理）；DeepSeek 可能保存数据，或用于改进它的模型，具体见它官网的隐私政策。",
                "随时可以关掉，也可以一键清空云端记录；密码、验证码这类输入框不会记录。",
            ],
        )),
        _ => None,
    }
}

/// 这个开关打开前要不要先过同意说明；要就弹，用户点「同意并开启」返回 true。
/// 已经开着的（用户要关它）不问。调用方拿到 true 才把这一项转发给素笺云。
pub fn confirm_consent(mtm: MainThreadMarker, feature: &str) -> bool {
    let Some((title, points)) = consent_points(feature) else {
        return true;
    };
    let app = NSApplication::sharedApplication(mtm);
    let previous = app.activationPolicy();
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    #[allow(deprecated)]
    app.activateIgnoringOtherApps(true);
    let body = format!(
        "{}\n\n点「同意并开启」表示你已阅读并同意以上说明。",
        points.join("\n\n")
    );
    let alert = message(mtm, title, &body, "同意并开启", "取消", true);
    let agreed = alert.runModal() == NSAlertFirstButtonReturn;
    app.setActivationPolicy(previous);
    agreed
}

/// 这三项要弹窗；其余 tag 直接转发给素笺云。
pub fn handles(tag: isize) -> bool {
    matches!(
        tag,
        TAG_CREATE_SPACE | TAG_JOIN_WITH_CODE | TAG_CLEAR_INPUT_LOG
    )
}

/// 弹窗并执行。调用方先问 [`handles`]。
pub fn run(tag: isize) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    // 输入法是 LSBackgroundOnly，弹窗前照偏好设置的做法临时切 Accessory 并激活，弹窗才拿得到焦点
    let app = NSApplication::sharedApplication(mtm);
    let previous = app.activationPolicy();
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    #[allow(deprecated)]
    app.activateIgnoringOtherApps(true);
    match tag {
        TAG_CREATE_SPACE => create_space(mtm),
        TAG_JOIN_WITH_CODE => join_with_code(mtm),
        TAG_CLEAR_INPUT_LOG => clear_input_log(mtm),
        _ => {}
    }
    app.setActivationPolicy(previous);
}

/// 「开通素笺云」：一句说明 + 出境同意，同意才建空间。
fn create_space(mtm: MainThreadMarker) {
    let alert = message(
        mtm,
        "开通素笺云服务",
        &format!("开通后在键盘上记的事会存到云端，每天整理成记忆卡，换设备也还在。\n\n{CONSENT}"),
        "同意并继续",
        "取消",
        true,
    );
    if alert.runModal() == NSAlertFirstButtonReturn {
        qingjian_cloud_mac::create_space(true);
    }
}

/// 「输入匹配码加入…」：说明 + 输入框 + 出境同意。
fn join_with_code(mtm: MainThreadMarker) {
    let alert = message(
        mtm,
        "加入已有的素笺云服务",
        &format!(
            "在手机上打开「我 → 素笺云服务 → 添加一台设备」，把那里显示的匹配码输进来。\n\n{CONSENT}"
        ),
        "加入",
        "取消",
        false,
    );
    let field = NSTextField::initWithFrame(
        mtm.alloc(),
        NSRect::new(NSPoint::ZERO, NSSize::new(220.0, 24.0)),
    );
    field.setPlaceholderString(Some(&NSString::from_str("匹配码")));
    alert.setAccessoryView(Some(&field));
    alert.window().setInitialFirstResponder(Some(&field));
    if alert.runModal() != NSAlertFirstButtonReturn {
        return;
    }
    let code = field.stringValue().to_string();
    if !code.trim().is_empty() {
        qingjian_cloud_mac::join_with_code(&code);
    }
}

/// 「清空云端输入记录…」：破坏性操作，先确认。
fn clear_input_log(mtm: MainThreadMarker) {
    let alert = message(
        mtm,
        "清空云端输入记录？",
        "服务器上已上传的全部输入记录会删掉，这台 Mac 的输入日志也一并清空。学到的词与设置不受影响。",
        "清空",
        "取消",
        true,
    );
    if alert.runModal() == NSAlertFirstButtonReturn {
        qingjian_cloud_mac::clear_input_log();
    }
}

/// 一个两按钮的提示框。确定的按钮排第一（返回 [`NSAlertFirstButtonReturn`]）。
fn message(
    mtm: MainThreadMarker,
    title: &str,
    body: &str,
    confirm: &str,
    cancel: &str,
    default_cancel: bool,
) -> objc2::rc::Retained<NSAlert> {
    let alert = NSAlert::new(mtm);
    alert.setMessageText(&NSString::from_str(title));
    alert.setInformativeText(&NSString::from_str(body));
    alert.addButtonWithTitle(&NSString::from_str(confirm));
    alert.addButtonWithTitle(&NSString::from_str(cancel));
    if default_cancel {
        // 默认按钮（回车）挪到「取消」上：这类要用户明确同意的操作，手一滑按回车不该等于同意。
        // 「取消」的标题让 AppKit 同时把 Esc 也挂到它上面。
        let buttons = alert.buttons();
        if let Some(confirm_button) = buttons.firstObject() {
            confirm_button.setKeyEquivalent(&NSString::from_str(""));
        }
        if let Some(cancel_button) = buttons.lastObject() {
            cancel_button.setKeyEquivalent(&NSString::from_str("\r"));
        }
    }
    alert
}
