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

    /// 登录前的激活策略，结束时还原（偏好设置窗口开着时是 Accessory，不能一律切回 Prohibited）。
    previous_policy: NSApplicationActivationPolicy,
}

impl WebLogin {
    /// 打开登录窗口；`url` 是 [`super::login_url`] 拼好的地址。
    pub fn start(
        mtm: MainThreadMarker,
        url: &str,
        verifier: String,
        device: String,
        generation: u64,
        sender: Sender<(u64, AccountEvent)>,
    ) -> Result<Self, String> {
        let url = NSURL::URLWithString(&NSString::from_str(url))
            .ok_or_else(|| "登录地址无效".to_owned())?;
        let handler: RcBlock<dyn Fn(*mut NSURL, *mut NSError)> =
            RcBlock::new(move |callback: *mut NSURL, error: *mut NSError| {
                // SAFETY: 系统给的指针要么为空要么在回调期间有效
                let (callback, error) = unsafe { (callback.as_ref(), error.as_ref()) };
                let event = match (callback, error) {
                    (Some(callback), _) => callback
                        .absoluteString()
                        .map(|text| AccountEvent::Callback(text.to_string()))
                        .unwrap_or_else(|| {
                            AccountEvent::LoginFailed("登录回调没有地址".to_owned())
                        }),
                    (None, Some(error))
                        if error.code() == ASWebAuthenticationSessionErrorCode::CanceledLogin.0 =>
                    {
                        AccountEvent::Canceled
                    }
                    (None, Some(error)) => AccountEvent::LoginFailed(format!(
                        "登录窗口出错：{}",
                        error.localizedDescription()
                    )),
                    (None, None) => AccountEvent::LoginFailed("登录没有完成".to_owned()),
                };
                let _ = sender.send((generation, event));
            });
        let scheme = NSString::from_str(LOGIN_CALLBACK_SCHEME);
        // initWithURL:callback:completionHandler: 要 macOS 14.4，输入法支持到 13
        // SAFETY: alloc 出来的对象立刻初始化；url、scheme 有效；completionHandler 是 `RcBlock::as_ptr`，
        // 系统会拷贝这个块，本函数返回后丢掉 `handler` 没问题
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
        // SAFETY: 锚点实现了协议；会话只弱引用它，`WebLogin` 握着 `anchor` 直到会话结束
        unsafe { session.setPresentationContextProvider(Some(ProtocolObject::from_ref(&*anchor))) };
        // 照偏好设置窗口的做法：切到 Accessory 并激活，登录窗才拿得到键盘焦点
        let app = NSApplication::sharedApplication(mtm);
        let previous_policy = app.activationPolicy();
        app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
        #[allow(deprecated)]
        app.activateIgnoringOtherApps(true);
        // SAFETY: 会话已配好展示上下文，在主线程上开始
        if !unsafe { session.start() } {
            anchor.close();
            app.setActivationPolicy(previous_policy);
            return Err("登录窗口打不开".to_owned());
        }
        Ok(Self {
            session,
            anchor,
            verifier,
            device,
            previous_policy,
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
    /// 取消会话、关锚点窗口，激活策略还原成登录前的样子。
    fn drop(&mut self) {
        // SAFETY: 会话对象仍然有效；cancel 对已结束的会话是空操作
        unsafe { self.session.cancel() };
        self.anchor.close();
        let mtm = MainThreadMarker::from(&*self.anchor);
        NSApplication::sharedApplication(mtm).setActivationPolicy(self.previous_policy);
    }
}
