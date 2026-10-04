//! 集成测试共用：定位产品数据。`QINGJIAN_REQUIRE_DATA=1` 时缺数据直接失败，不静默跳过。

use std::path::PathBuf;

/// `QINGJIAN_DATA` 指向含 `dict.qj` 的目录，必须是**绝对路径**（测试的工作目录是 crate 目录，相对路径解析不到）。
/// - 没设 `QINGJIAN_DATA`：默认返回 `None` 让测试跳过；设了 `QINGJIAN_REQUIRE_DATA=1`（CI、提交前）则 panic。
/// - 设了但无效（相对路径、缺 `dict.qj`）：一律 panic。指错路径若静默跳过，测试 0.00 秒"通过"，等于没测。
pub fn data_dir() -> Option<PathBuf> {
    let Some(dir) = std::env::var_os("QINGJIAN_DATA").map(PathBuf::from) else {
        if std::env::var_os("QINGJIAN_REQUIRE_DATA").is_some_and(|v| v == "1") {
            panic!(
                "QINGJIAN_REQUIRE_DATA=1 但没设 QINGJIAN_DATA：要指向含 dict.qj 的目录（绝对路径）"
            );
        }
        eprintln!("没有 QINGJIAN_DATA，跳过");
        return None;
    };
    assert!(
        dir.is_absolute(),
        "QINGJIAN_DATA 要用绝对路径（测试在 crate 目录下跑，相对路径解析不到）：{}",
        dir.display()
    );
    assert!(
        dir.join("dict.qj").is_file(),
        "QINGJIAN_DATA 指向的目录里没有 dict.qj：{}",
        dir.display()
    );
    Some(dir)
}
