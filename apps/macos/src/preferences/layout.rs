use objc2::Message;
use objc2::rc::Retained;
use objc2_app_kit::NSView;
use objc2_foundation::{NSPoint, NSRect, NSSize};

/// 行高。
pub const ROW_HEIGHT: f64 = 20.0;

/// 行距。
pub const ROW_GAP: f64 = 4.0;

/// 页内左右留白。
pub const PAGE_PADDING: f64 = 20.0;

/// 标题列的宽度（标题右对齐贴着控件）。
pub const LABEL_WIDTH: f64 = 110.0;

/// 控件列的 x：标题列 + 间隔。
pub const CONTROL_X: f64 = PAGE_PADDING + LABEL_WIDTH + 10.0;

/// 每个标签页的内容宽度。
pub const PAGE_WIDTH: f64 = 600.0;

/// 在一页里自上而下摆控件的简易布局：承载视图是**翻转坐标**（原点在左上，见
/// [`crate::preferences::flipped::FlippedView`]），所以「离顶部多远」就是 frame 的 y，
/// 页高怎么变都不用重算已经摆好的控件。
#[derive(Clone)]
pub struct Layout {
    /// 这一页的宽度。
    width: f64,

    /// 已摆的控件及其（x, 顶部距离, 宽, 高, 是否撑到页底）。
    placed: Vec<(Retained<NSView>, f64, f64, f64, f64, bool)>,

    /// 当前行的顶部距离。
    top: f64,
}

impl Layout {
    pub fn new(width: f64, top_margin: f64) -> Self {
        Self {
            width,
            placed: Vec::new(),
            top: top_margin,
        }
    }

    /// 控件列从 [`CONTROL_X`] 到右边留白之间的宽度。
    pub fn control_width(&self) -> f64 {
        self.width - CONTROL_X - PAGE_PADDING
    }

    /// 从左留白到右留白的整行宽度。
    pub fn inner_width(&self) -> f64 {
        self.width - 2.0 * PAGE_PADDING
    }

    /// 在当前行放一个控件。
    pub fn place(&mut self, view: &NSView, x: f64, width: f64, height: f64) {
        self.placed
            .push((view.retain(), x, self.top, width, height, false));
    }

    /// 在当前行放一个控件，页高定下来后撑到页底（至少 `min_height`）；窗口比这一页高时不留空。
    pub fn place_fill(&mut self, view: &NSView, x: f64, width: f64, min_height: f64) {
        self.placed
            .push((view.retain(), x, self.top, width, min_height, true));
    }

    /// 换到下一行。
    pub fn next_row(&mut self, height: f64) {
        self.top += height + ROW_GAP;
    }

    /// 额外留白（分组之间）。
    pub fn space(&mut self, height: f64) {
        self.top += height;
    }

    /// 刚摆下的那个控件（说明小字改成悬停提示时挂它身上）。
    pub fn last_placed(&self) -> Option<&NSView> {
        self.placed.last().map(|(view, ..)| &**view)
    }

    /// 到目前为止用掉的高度（含顶部留白）。
    pub fn height(&self) -> f64 {
        self.top
    }

    /// 把所有控件加进容器并按容器高度设好 frame（顶部对齐）。
    /// 容器是翻转坐标，摆的位置跟页高无关；页高变了只有撑到页底的那几个要重算。
    pub fn finish(&self, container: &NSView, total_height: f64) {
        for (view, x, top, width, height, fill) in &self.placed {
            let height = if *fill {
                (total_height - top - PAGE_PADDING).max(*height)
            } else {
                *height
            };
            let y = *top;
            view.setFrame(NSRect::new(
                NSPoint::new(*x, y),
                NSSize::new(*width, height),
            ));
            container.addSubview(view);
        }
    }
}
