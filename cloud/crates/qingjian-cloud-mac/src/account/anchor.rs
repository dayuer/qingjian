//! `ASWebAuthenticationSession` 要的展示锚点。输入法是 LSBackgroundOnly 进程，平时没有窗口：
//! 登录时临时建一个无边框、透明的 1×1 窗口放在主屏中央当锚点，登录结束由 `WebLogin` 关掉。
//! 会话的 presentationContextProvider 是弱引用，`WebLogin` 要自己握着本对象。

use objc2::rc::Retained;
use objc2::runtime::NSObject;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{NSBackingStoreType, NSWindow, NSWindowStyleMask};
use objc2_authentication_services::{
    ASPresentationAnchor, ASWebAuthenticationPresentationContextProviding,
    ASWebAuthenticationSession,
};
use objc2_foundation::{NSObjectProtocol, NSPoint, NSRect, NSSize};

define_class!(
    // SAFETY: NSObject 允许子类化；没有实现 Drop。
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = Retained<NSWindow>]
    pub struct LoginAnchor;

    unsafe impl NSObjectProtocol for LoginAnchor {}

    unsafe impl ASWebAuthenticationPresentationContextProviding for LoginAnchor {
        #[unsafe(method_id(presentationAnchorForWebAuthenticationSession:))]
        fn presentation_anchor(
            &self,
            _session: &ASWebAuthenticationSession,
        ) -> Retained<ASPresentationAnchor> {
            // ASPresentationAnchor 在绑定里是 NSObject：NSWindow → NSResponder → NSObject
            Retained::into_super(Retained::into_super(self.ivars().clone()))
        }
    }
);

impl LoginAnchor {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1.0, 1.0));
        let window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                mtm.alloc::<NSWindow>(),
                frame,
                NSWindowStyleMask::Borderless,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        // 程序建的 NSWindow 默认关窗即释放，我们还握着 Retained，必须关掉
        unsafe { window.setReleasedWhenClosed(false) };
        window.setAlphaValue(0.0);
        window.center();
        window.orderFrontRegardless();
        let this = mtm.alloc::<Self>().set_ivars(window);
        unsafe { msg_send![super(this), init] }
    }

    pub fn close(&self) {
        self.ivars().close();
    }
}
