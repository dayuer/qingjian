//! 网页登录：用 `ASWebAuthenticationSession` 打开服务端的登录页（Apple 与邮箱都在页面上），
//! 等它回跳 `sujian://auth?handoff=…`。结果经通道交给主线程拍子；丢掉本对象就取消登录、关掉锚点窗口。

use std::sync::mpsc::Sender;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::ProtocolObject;
use objc2::{AnyThread, MainThreadMarker};
use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
use objc2_authentication_services::{
    ASWebAuthenticationSession, ASWebAuthenticationSessionErrorCode,
};
use objc2_foundation::{NSError, NSString, NSURL};
use qingjian_cloud_proto::LOGIN_CALLBACK_SCHEME;

use super::{AccountEvent, LoginAnchor};

pub struct WebLogin {
    session: Retained<ASWebAuthenticationSession>,

    anchor: Retained<LoginAnchor>,

    /// PKCE 的 verifier：只在本进程里，换令牌时交给服务端核对。
    verifier: String,

    /// 报给服务端的设备名。
    device: String,
}

impl WebLogin {
    /// 打开登录窗口；`url` 是 [`super::login_url`] 拼好的地址。
    pub fn start(
        mtm: MainThreadMarker,
        url: &str,
        verifier: String,
        device: String,
        sender: Sender<AccountEvent>,
    ) -> Result<Self, String> {
        let url = NSURL::URLWithString(&NSString::from_str(url))
            .ok_or_else(|| "登录地址无效".to_owned())?;
        let handler: RcBlock<dyn Fn(*mut NSURL, *mut NSError)> =
            RcBlock::new(move |callback: *mut NSURL, error: *mut NSError| {
                // SAFETY: 系统给的指针要么为空要么在回调期间有效
                let (callback, error) = unsafe { (callback.as_ref(), error.as_ref()) };
                let result = match (callback, error) {
                    (Some(callback), _) => callback
                        .absoluteString()
                        .map(|text| text.to_string())
                        .ok_or_else(|| "登录回调没有地址".to_owned()),
                    (None, Some(error))
                        if error.code() == ASWebAuthenticationSessionErrorCode::CanceledLogin.0 =>
                    {
                        Err("已取消登录".to_owned())
                    }
                    (None, Some(error)) => {
                        Err(format!("登录窗口出错：{}", error.localizedDescription()))
                    }
                    (None, None) => Err("登录没有完成".to_owned()),
                };
                let _ = sender.send(AccountEvent::Callback(result));
            });
        let scheme = NSString::from_str(LOGIN_CALLBACK_SCHEME);
        // initWithURL:callback:completionHandler: 要 macOS 14.4，输入法支持到 13
        #[allow(deprecated)]
        let session = unsafe {
            ASWebAuthenticationSession::initWithURL_callbackURLScheme_completionHandler(
                ASWebAuthenticationSession::alloc(),
                &url,
                Some(&scheme),
                RcBlock::as_ptr(&handler),
            )
        };
        let anchor = LoginAnchor::new(mtm);
        unsafe { session.setPresentationContextProvider(Some(ProtocolObject::from_ref(&*anchor))) };
        // 照偏好设置窗口的做法：切到 Accessory 并激活，登录窗才拿得到键盘焦点
        let app = NSApplication::sharedApplication(mtm);
        app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
        #[allow(deprecated)]
        app.activateIgnoringOtherApps(true);
        if !unsafe { session.start() } {
            anchor.close();
            app.setActivationPolicy(NSApplicationActivationPolicy::Prohibited);
            return Err("登录窗口打不开".to_owned());
        }
        Ok(Self {
            session,
            anchor,
            verifier,
            device,
        })
    }

    pub fn verifier(&self) -> &str {
        &self.verifier
    }

    pub fn device(&self) -> &str {
        &self.device
    }
}

impl Drop for WebLogin {
    /// 已经结束的会话 cancel 是空操作；关锚点窗口，输入法回到纯后台。
    fn drop(&mut self) {
        unsafe { self.session.cancel() };
        self.anchor.close();
        let mtm = MainThreadMarker::from(&*self.anchor);
        NSApplication::sharedApplication(mtm)
            .setActivationPolicy(NSApplicationActivationPolicy::Prohibited);
    }
}
