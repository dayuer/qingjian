//! 0.5 秒一拍的定时器 target。输入法自己的定时器在失焦（deactivateServer）时就停，同步得有自己的。

use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_foundation::{NSObject, NSObjectProtocol};

use crate::service;

define_class!(
    // SAFETY: NSObject 没有子类化要求；没有实现 Drop。
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    #[name = "QingjianCloudTimerTarget"]
    pub struct TimerTarget;

    impl TimerTarget {
        #[unsafe(method(tick:))]
        fn tick(&self, _timer: Option<&AnyObject>) {
            service::tick();
        }
    }

    unsafe impl NSObjectProtocol for TimerTarget {}
);

impl TimerTarget {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}
