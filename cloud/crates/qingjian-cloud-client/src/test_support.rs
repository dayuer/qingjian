//! crate 内测试与依赖方集成测试共用的假服务，源码与 `tests/` 下的集成测试是同一份。

#[path = "../tests/support/mod.rs"]
mod support;

pub use support::{
    fake_server, fake_server_sequence, fake_server_with_body, has_authorization, request_line,
};
