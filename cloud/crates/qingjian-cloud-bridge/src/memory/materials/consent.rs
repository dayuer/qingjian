//! 2B 同意页里「记忆」那一项：没同意过、同意了、同意后又撤回。撤回要和没同意分开，因为它要停掉已经排着的上传。

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Consent {
    NotGiven,

    Given,

    Withdrawn,
}
