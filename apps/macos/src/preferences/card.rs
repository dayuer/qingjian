//! 分组卡片：系统设置那种分组框——浅色圆角底，组标题在框**外**上方（小号灰字），
//! 组里一行一项：左边名称（下面可挂一行说明小字），右边控件（开关、按钮）右对齐。
//! 卡片自己算高度，调用方只管往里加行，最后 [`Card::finish`] 把它摆进页面。

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSBox, NSBoxType, NSColor, NSFont, NSTextField, NSView};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

use super::layout::{Layout, ROW_GAP};

/// 卡片内的左右内边距。
const PAD_H: f64 = 14.0;

/// 一行里控件之间的间距。
const GAP: f64 = 8.0;

/// 一行里控件的高（按钮、开关、弹出菜单都一样）。
const CONTROL_H: f64 = 22.0;

/// 卡片内的上下内边距。
const PAD_V: f64 = 10.0;

/// 行的名称行高。
const NAME_H: f64 = 20.0;

/// 说明小字的行高。
const NOTE_H: f64 = 14.0;

/// 卡片圆角。
const CORNER: f64 = 8.0;

/// 说明小字按多少像素一个字估算折行（11 号字，中文约 11 px，估得宽一点宁可多留一行）。
const NOTE_CHAR_WIDTH: f64 = 11.5;

pub struct Card {
    /// 卡片本体。
    view: Retained<NSBox>,

    /// 已摆的控件：(控件, x, 距卡片顶部, 宽, 高)。
    rows: Vec<(Retained<NSView>, f64, f64, f64, f64)>,

    /// 卡片宽度。
    width: f64,

    /// 下一个控件的顶部距离。
    top: f64,
}

impl Card {
    /// 建一张卡片；组标题（小号灰字）要调用方自己 place 到页面上方。
    pub fn new(mtm: MainThreadMarker, width: f64, title: &str) -> (Retained<NSTextField>, Self) {
        let view = NSBox::new(mtm);
        view.setBoxType(NSBoxType::Custom);
        view.setBorderWidth(0.0);
        view.setCornerRadius(CORNER);
        view.setTransparent(false);
        // 窗口底是白的，controlBackgroundColor 也是白的，卡片会看不见；
        // 用比底色差一档的填充：浅色下浅灰、深色下比窗口底亮一档
        view.setFillColor(&NSColor::quaternaryLabelColor());
        view.setContentViewMargins(NSSize::new(0.0, 0.0));

        let label = NSTextField::labelWithString(&NSString::from_str(title), mtm);
        label.setFont(Some(&NSFont::systemFontOfSize(11.0)));
        label.setTextColor(Some(&NSColor::secondaryLabelColor()));

        (
            label,
            Self {
                view,
                rows: Vec::new(),
                width,
                top: PAD_V,
            },
        )
    }

    /// 控件列的宽度（开关固定 40、按钮按标题算）。
    fn control_width(&self) -> f64 {
        self.width - 2.0 * PAD_H
    }

    /// 加一行：左边名称（+可选说明），右边一个控件（右对齐）。交出两个标签，按状态改文字时用。
    /// `control_width` 是控件的宽（开关固定、按钮按标题算）。
    pub fn row(
        &mut self,
        mtm: MainThreadMarker,
        name: &str,
        note: Option<&str>,
        control: &NSView,
        control_width: f64,
    ) -> (Retained<NSTextField>, Option<Retained<NSTextField>>) {
        self.row_with_buttons(mtm, name, note, &[(control, control_width)])
    }

    /// 同 [`Self::row`]，右边可以放几颗按钮。
    pub fn row_with_buttons(
        &mut self,
        mtm: MainThreadMarker,
        name: &str,
        note: Option<&str>,
        controls: &[(&NSView, f64)],
    ) -> (Retained<NSTextField>, Option<Retained<NSTextField>>) {
        self.add_row(mtm, name, note, controls, NAME_H)
    }

    /// 同 [`Self::row_with_buttons`]，名称那一行留 `name_h` 高（状态行、同步行的文字会长到两三行）。
    pub fn row_tall(
        &mut self,
        mtm: MainThreadMarker,
        name: &str,
        note: Option<&str>,
        controls: &[(&NSView, f64)],
        name_h: f64,
    ) -> (Retained<NSTextField>, Option<Retained<NSTextField>>) {
        self.add_row(mtm, name, note, controls, name_h)
    }

    /// 同 [`Self::row_tall`]，再交出这一行的位置（[`RowSlot`]），
    /// 供两套控件轮流上场的行（状态行的「开通 / 加入」与「解绑」）按状态重新右对齐。
    pub fn row_group(
        &mut self,
        mtm: MainThreadMarker,
        name: &str,
        note: Option<&str>,
        controls: &[(&NSView, f64)],
        name_h: f64,
    ) -> (
        Retained<NSTextField>,
        Option<Retained<NSTextField>>,
        RowSlot,
    ) {
        let slot = RowSlot {
            top: self.top,
            width: self.width,
        };
        let (name_label, note_label) = self.add_row(mtm, name, note, controls, name_h);
        (name_label, note_label, slot)
    }

    /// 一行：左边名称（可折行，留 `name_h` 高）与说明小字，右边一组控件右对齐排。
    fn add_row(
        &mut self,
        mtm: MainThreadMarker,
        name: &str,
        note: Option<&str>,
        controls: &[(&NSView, f64)],
        name_h: f64,
    ) -> (Retained<NSTextField>, Option<Retained<NSTextField>>) {
        let total: f64 = controls.iter().map(|(_, w)| w + GAP).sum::<f64>() - GAP;
        let text_width = (self.control_width() - total - 12.0).max(80.0);
        let name_label = label(mtm, name, 13.0, None);
        self.rows.push((
            as_view(name_label.clone()),
            PAD_H,
            self.top,
            text_width,
            name_h,
        ));

        let mut height = name_h;
        let mut note_label = None;
        if let Some(note) = note {
            let note_h = note_height(note, text_width);
            let label = label(mtm, note, 11.0, Some(&NSColor::secondaryLabelColor()));
            self.rows.push((
                as_view(label.clone()),
                PAD_H,
                self.top + name_h - 2.0,
                text_width,
                note_h,
            ));
            height = name_h + note_h - 2.0;
            note_label = Some(label);
        }
        // 控件垂直居中对着名称那一块，整体右对齐
        let mut x = self.width - PAD_H - total;
        for (control, width) in controls {
            let control_top = self.top + (name_h - CONTROL_H).max(0.0) / 2.0;
            self.rows
                .push((as_view_any(control), x, control_top, *width, CONTROL_H));
            x += width + GAP;
        }

        self.top += height + ROW_GAP;
        (name_label, note_label)
    }

    /// 往 [`Self::row_group`] 那一行再塞一个控件：它只在别的状态下出现，位置由
    /// [`RowSlot::align_right`] 定，不参与文字列宽的计算（否则文字列会被挤没）。
    pub fn row_extra(&mut self, slot: &RowSlot, view: &NSView, width: f64) {
        self.rows
            .push((as_view_any(view), PAD_H, slot.top, width, CONTROL_H));
    }

    /// 加一行整行宽的说明小字（按钮下面那种），左对齐、放不下折行。
    pub fn row_note(&mut self, mtm: MainThreadMarker, text: &str) {
        let text_width = self.control_width();
        let height = note_height(text, text_width);
        let label = label(mtm, text, 11.0, Some(&NSColor::secondaryLabelColor()));
        self.rows
            .push((as_view(label), PAD_H, self.top, text_width, height));
        self.top += height + ROW_GAP;
    }

    /// 卡片本体（要整张藏起来时先留一份）。
    pub fn box_view(&self) -> Retained<NSView> {
        as_view(self.view.clone())
    }

    /// 卡片高度（含内边距）。
    pub fn height(&self) -> f64 {
        (self.top - ROW_GAP + PAD_V).max(PAD_V * 2.0 + NAME_H)
    }

    /// 把卡片摆进页面（交给布局器加进容器），行在卡片里定位；交回卡片高度，
    /// 行里位置随时在变的控件（[`RowSlot::align_right`]）要拿它换算。
    pub fn finish(self, layout: &mut Layout) -> f64 {
        let height = self.height();
        layout.place(&self.view, super::layout::PAGE_PADDING, self.width, height);
        layout.next_row(height);

        for (view, x, top, w, h) in self.rows {
            // AppKit 原点在左下：按卡片高度换算
            let y = height - top - h;
            view.setFrame(NSRect::new(NSPoint::new(x, y), NSSize::new(w, h)));
            self.view.addSubview(&view);
        }
        height
    }
}

/// 卡片里某一行的位置，卡片摆好后用来重排这一行右边的控件。
pub struct RowSlot {
    /// 距卡片顶部。
    top: f64,

    /// 卡片宽度。
    width: f64,
}

impl RowSlot {
    /// 把可见的控件在这一行里右对齐排好；`card_height` 是 [`Card::finish`] 交回的卡片高度。
    pub fn align_right(&self, card_height: f64, items: &[(&NSView, f64)]) {
        let total: f64 = items.iter().map(|(_, w)| w + GAP).sum::<f64>() - GAP;
        let y = card_height - self.top - CONTROL_H;
        let mut x = (self.width - PAD_H - total).max(PAD_H);
        for (view, width) in items {
            view.setFrame(NSRect::new(
                NSPoint::new(x, y),
                NSSize::new(*width, CONTROL_H),
            ));
            x += width + GAP;
        }
    }
}

/// 一行说明小字的高度：按文字列宽折算行数（宁可多留一行）。
fn note_height(text: &str, width: f64) -> f64 {
    let estimated = text.chars().count() as f64 * NOTE_CHAR_WIDTH;
    let lines = (estimated / width).ceil().max(1.0);
    NOTE_H * lines
}

/// 把控件当 `&NSView` 用（[`RowSlot::align_right`] 要拿它重排位置）。
pub fn view<T: objc2::Message + 'static>(value: &Retained<T>) -> &NSView {
    // SAFETY: 调用点传的都是 NSTextField / NSButton / NSSwitch，全是 NSView 的子类
    unsafe { &*(Retained::as_ptr(value) as *const NSView) }
}

/// 标签与控件统一按 `NSView` 存（`NSTextField` / `NSButton` / `NSSwitch` 都是它的子类）。
/// `cast_unchecked` 的类型安全性由这里保证：传进来的都是 NSView 家族的 AppKit 控件。
fn as_view<T: objc2::Message + 'static>(view: Retained<T>) -> Retained<NSView> {
    // SAFETY: 调用点传的都是 NSTextField / NSButton / NSSwitch，全是 NSView 的子类
    unsafe { Retained::cast_unchecked(view) }
}

/// 已经是 `&NSView` 的控件（按钮、开关这类）复制一份引用。
fn as_view_any(view: &NSView) -> Retained<NSView> {
    unsafe { Retained::retain(view as *const NSView as *mut NSView) }.expect("控件还活着")
}

/// 一行文字的标签；`color` 为 `None` 时用默认（正文）色。
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
    label.setUsesSingleLineMode(false);
    if let Some(cell) = label.cell() {
        cell.setWraps(true);
    }
    label
}

/// 开关一行用的固定宽（NSSwitch 的固有宽度）。
pub const SWITCH_WIDTH: f64 = 42.0;
