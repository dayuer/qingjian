//! 集成测试共用：定位产品数据。`QINGJIAN_REQUIRE_DATA=1` 时缺数据直接失败，不静默跳过。

use std::path::PathBuf;

/// `QINGJIAN_DATA` 指向含 `dict.qj` 的目录。缺数据：默认返回 `None` 让测试跳过；
/// 设了 `QINGJIAN_REQUIRE_DATA=1`（CI、提交前）则 panic。
pub fn data_dir() -> Option<PathBuf> {
    let found = std::env::var_os("QINGJIAN_DATA")
        .map(PathBuf::from)
        .filter(|dir| dir.join("dict.qj").is_file());
    if found.is_none() {
        if std::env::var_os("QINGJIAN_REQUIRE_DATA").is_some_and(|v| v == "1") {
            panic!("QINGJIAN_REQUIRE_DATA=1 但没有产品数据：QINGJIAN_DATA 要指向含 dict.qj 的目录");
        }
        eprintln!("没有 QINGJIAN_DATA（或缺 dict.qj），跳过");
    }
    found
}
