//! 当前输入法是不是青简：本程序只在用青简时运行，切到别的输入法就退出，切回来由输入法拉起（见 `app.rs`）。
//! 走 Carbon 的 Text Input Source Services；CFString 与 NSString 是免费桥接的，直接当 NSString 读。

use std::ffi::c_void;

use objc2_foundation::NSString;

/// 青简的输入源 ID 前缀（输入法本体与它的输入模式 `.Hans` 都以它开头）。
const QINGJIAN_SOURCE: &str = "app.qingjian.inputmethod";

#[link(name = "Carbon", kind = "framework")]
unsafe extern "C" {
    /// 当前的键盘输入源；返回值归调用方释放。
    fn TISCopyCurrentKeyboardInputSource() -> *const c_void;

    /// 读输入源的一个属性；返回值归系统，不释放。
    fn TISGetInputSourceProperty(source: *const c_void, key: *const c_void) -> *const c_void;

    /// 属性键：输入源 ID。
    static kTISPropertyInputSourceID: *const c_void;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(object: *const c_void);
}

/// 当前输入法是青简；读不到（登录窗口、系统还没就绪）时返回 `None`，调用方别据此退出。
pub fn qingjian_selected() -> Option<bool> {
    // SAFETY: TIS 函数在主线程调用；Copy 出来的对象用完 CFRelease，Get 出来的属性不释放
    unsafe {
        let source = TISCopyCurrentKeyboardInputSource();
        if source.is_null() {
            return None;
        }
        let id = TISGetInputSourceProperty(source, kTISPropertyInputSourceID);
        let selected = (!id.is_null()).then(|| {
            let id = &*(id as *const NSString);
            id.to_string().starts_with(QINGJIAN_SOURCE)
        });
        CFRelease(source);
        selected
    }
}
