//! 菜单项与定时器的 target：常驻程序没有 key window，nil target 走不到响应链，得有个实体对象收 action。

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::NSMenuItem;
use objc2_foundation::{NSObject, NSObjectProtocol};

use crate::app;

define_class!(
    // SAFETY: NSObject 没有子类化要求；没有实现 Drop。
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    pub struct Target;

    impl Target {
        #[unsafe(method(menuAction:))]
        fn menu_action(&self, sender: Option<&NSMenuItem>) {
            if let Some(item) = sender {
                app::with(|app| app.perform(item.tag()));
            }
        }

        #[unsafe(method(tick:))]
        fn tick(&self, _timer: Option<&AnyObject>) {
            app::with(|app| app.tick());
        }
    }

    unsafe impl NSObjectProtocol for Target {}
);

impl Target {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}
