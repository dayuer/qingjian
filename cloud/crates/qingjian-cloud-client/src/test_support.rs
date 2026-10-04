//! crate 内测试用的假服务，源码与 `tests/` 下的集成测试共用同一份。

#[path = "../tests/support/mod.rs"]
mod support;

pub use support::fake_server;
