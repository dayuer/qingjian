//! 补传素材时看的素笺云状态：只分开通没有（`cloud.toml` 里有没有令牌）。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudState {
    NotActivated,

    Activated,
}
