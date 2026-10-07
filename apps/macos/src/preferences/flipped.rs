//! 页面的承载视图：坐标原点在**左上角**。
//!
//! 设置页的控件都是自上而下排的（见 `layout.rs`），用翻转坐标有两个好处：
//! 页高变了不用重算已经摆好的控件（收起一块只影响下面那几块的页高），
//! 滚动视图里 `scrollToPoint(0, 0)` 就是页顶，不用拿页高去反算。

use objc2::rc::Retained;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::NSView;
use objc2_foundation::{NSObjectProtocol, NSRect};

define_class!(
    // SAFETY: NSView 允许子类化；没有实现 Drop。
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    pub struct FlippedView;

    impl FlippedView {
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }
    }

    unsafe impl NSObjectProtocol for FlippedView {}
);

impl FlippedView {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        unsafe { msg_send![super(this), initWithFrame: NSRect::ZERO] }
    }
}

/// 当普通 `NSView` 用（布局器只认它）。
pub fn view_of(view: &Retained<FlippedView>) -> Retained<NSView> {
    // SAFETY: FlippedView 是 NSView 的子类
    unsafe { Retained::cast_unchecked(view.clone()) }
}
