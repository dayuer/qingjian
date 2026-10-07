//! 「设备」那一块：同一空间里**别的**设备一行一台——名字、「iOS · 3 分钟前活跃」，
//! 右边一颗红字「解绑…」（点了先弹确认，见 `PreferencesTarget::revokeDevice:`）。
//! 本机不占这里：状态行上写着它的名字，解绑也在那一行。
//! 行是动态的（别的设备加入 / 被解绑都要跟着变），所以不摆进卡片的行里，
//! 而是自己占一块固定高度的区域，按状态重建；设备多了在区域里滚。

use std::cell::RefCell;

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::sel;
use objc2_app_kit::{NSColor, NSFont, NSScrollView, NSTextField, NSView};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use qingjian_cloud_mac::CloudDevice;

use crate::preferences::card::retained_view;
use crate::preferences::controls::{action_danger_button, small_label};
use crate::preferences::target::PreferencesTarget;

/// 列表在卡片里占的高度（约两台，再多自己滚）。
pub const HEIGHT: f64 = 2.0 * ROW + 4.0;

/// 一行的高。
const ROW: f64 = 34.0;

/// 「解绑…」按钮的宽。
const BUTTON_W: f64 = 84.0;

pub struct DeviceList {
    /// 装着行的滚动视图（固定高度，设备多了滚）。
    scroll: Retained<NSScrollView>,

    /// 文档视图，行都加在它上面。
    list: Retained<NSView>,

    /// 当前的行控件，重建时先移除。
    rows: RefCell<Vec<Retained<NSView>>>,

    /// 一台设备都没有时的说明。
    empty: Retained<NSTextField>,

    /// 行里「解绑…」按钮的 target。
    target: Retained<PreferencesTarget>,

    mtm: MainThreadMarker,

    /// 行区域的宽。
    width: f64,
}

impl DeviceList {
    pub fn new(mtm: MainThreadMarker, width: f64, target: &Retained<PreferencesTarget>) -> Self {
        let list = NSView::initWithFrame(mtm.alloc(), NSRect::ZERO);
        let scroll = NSScrollView::initWithFrame(
            mtm.alloc(),
            NSRect::new(NSPoint::ZERO, NSSize::new(width, HEIGHT)),
        );
        scroll.setHasVerticalScroller(true);
        scroll.setDrawsBackground(false);
        scroll.setDocumentView(Some(&list));
        let empty = small_label(mtm, "");
        list.addSubview(&empty);
        Self {
            scroll,
            list,
            rows: RefCell::new(Vec::new()),
            empty,
            target: target.clone(),
            mtm,
            width,
        }
    }

    /// 摆进卡片那一行。
    pub fn view(&self) -> &NSView {
        &self.scroll
    }

    /// 按当前设备清单重建（本机不列）。`signed_in` 只用来决定空列表说什么。
    pub fn rebuild(&self, devices: &[CloudDevice], signed_in: bool) {
        let devices: Vec<&CloudDevice> = devices.iter().filter(|device| !device.current).collect();
        for view in self.rows.borrow_mut().drain(..) {
            view.removeFromSuperview();
        }
        // 文档视图按行数撑高（至少一屏）；行从顶部往下排
        let height = (devices.len() as f64 * ROW).max(HEIGHT);
        self.list
            .setFrame(NSRect::new(NSPoint::ZERO, NSSize::new(self.width, height)));
        self.empty
            .setTextColor(Some(&NSColor::secondaryLabelColor()));
        self.empty.setStringValue(&NSString::from_str(if signed_in {
            "还没有别的设备：在手机上用「添加一台设备」出码，新设备输码加入。"
        } else {
            "开通后这里列出同一空间里的其他设备，可以在这里解绑。"
        }));
        self.empty.setHidden(!devices.is_empty());
        self.empty.setFrame(NSRect::new(
            NSPoint::new(0.0, height - 18.0),
            NSSize::new(self.width, 16.0),
        ));
        let mut rows = self.rows.borrow_mut();
        for (index, device) in devices.iter().enumerate() {
            let y = height - index as f64 * ROW - ROW;
            let text_width = self.width - BUTTON_W - 10.0;
            let name = label(self.mtm, &device.name, 13.0, None);
            name.setFrame(NSRect::new(
                NSPoint::new(0.0, y + 16.0),
                NSSize::new(text_width, 18.0),
            ));
            let detail = small_label(self.mtm, &device.detail);
            detail.setFrame(NSRect::new(
                NSPoint::new(0.0, y + 1.0),
                NSSize::new(text_width, 14.0),
            ));
            self.list.addSubview(&name);
            self.list.addSubview(&detail);
            rows.push(retained_view(name));
            rows.push(retained_view(detail));
            let unbind = action_danger_button(
                self.mtm,
                "解绑…",
                sel!(revokeDevice:),
                device.id as isize,
                &self.target,
            );
            unbind.setFrame(NSRect::new(
                NSPoint::new(self.width - BUTTON_W, y + 6.0),
                NSSize::new(BUTTON_W, 22.0),
            ));
            self.list.addSubview(&unbind);
            rows.push(retained_view(unbind));
        }
    }
}

/// 一行文字的标签（正文色）。
fn label(
    mtm: MainThreadMarker,
    text: &str,
    size: f64,
    color: Option<&NSColor>,
) -> Retained<NSTextField> {
    let label = NSTextField::labelWithString(&NSString::from_str(text), mtm);
    label.setFont(Some(&NSFont::systemFontOfSize(size)));
    if let Some(color) = color {
        label.setTextColor(Some(color));
    }
    label
}
