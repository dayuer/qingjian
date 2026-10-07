use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2::{MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_foundation::{NSObject, NSObjectProtocol};

use crate::host;

define_class!(
    // SAFETY: NSObject 没有子类化要求；没有实现 Drop。
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[ivars = ()]
    /// 设置窗口所有控件的 target：一个 `changed:` 选择器，靠 tag 区分控件。
    pub struct PreferencesTarget;

    impl PreferencesTarget {
        #[unsafe(method(editPhrase:))]
        fn edit_phrase(&self, _sender: Option<&AnyObject>) {
            host::with(|h| h.change_setting(super::Setting::EditPhrase, super::SettingValue::Bool(false)));
        }

        /// 「云服务」页设备列表里点了一台设备的「解绑…」：tag 是会话 id，先弹确认再解绑。
        #[unsafe(method(revokeDevice:))]
        fn revoke_device(&self, sender: Option<&AnyObject>) {
            let Some(sender) = sender else {
                return;
            };
            // SAFETY: 调用点都是 NSButton（NSControl），tag 是它在设备列表里的会话 id
            let id: isize = unsafe { msg_send![sender, tag] };
            host::with(|h| h.revoke_cloud_device(id as i64));
        }

        /// 「云服务」页「高级」的展开三角：只改界面，不动配置。
        #[unsafe(method(toggleCloudAdvanced:))]
        fn toggle_cloud_advanced(&self, _sender: Option<&AnyObject>) {
            host::with(|h| h.toggle_cloud_advanced());
        }

        #[unsafe(method(changed:))]
        fn changed(&self, sender: Option<&AnyObject>) {
            if let Some((setting, value)) = super::setting_from_sender(sender) {
                host::with(|h| h.change_setting(setting, value));
            }
        }
    }

    unsafe impl NSObjectProtocol for PreferencesTarget {}
);

impl PreferencesTarget {
    pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc::<Self>().set_ivars(());
        unsafe { msg_send![super(this), init] }
    }
}
