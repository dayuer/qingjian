//! 三种修正：词库体检（审用户词的读音与垃圾词）、一次就学会（分几次选完的词）、话题补词（预测要用到但打不出来的词）。

mod audit;
mod compose;
mod topics;

pub use audit::audit;
pub use compose::from_compositions;
pub use topics::topics;
