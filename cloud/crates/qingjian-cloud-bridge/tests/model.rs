//! 本地神经模型的加载 / 卸载 / 状态：`QINGJIAN_DATA` 指向含 `dict.qj` 的目录，
//! 模型按 `data_dir/models/…` 与 `data_dir/../models/…` 两处找（生成数据在 `data/generated`，模型在 `data/models`）。
//! 没给或缺文件就跳过（`QINGJIAN_REQUIRE_DATA=1` 时失败），惯例同 `tests/session.rs`。

mod support;

use std::path::{Path, PathBuf};

use self::support::data_dir;
use qingjian_cloud_bridge::{
    Entry, MODEL_ACTIVE, MODEL_FAILED, MODEL_IDLE, MODEL_LOADING, Session,
};
use qingjian_core::CandidateKind;

/// 找通变模型：`QJ_MODEL` 优先（比如随包的 8 位 `.qjm`），再看两处默认位置；都没有返回 `None`（测试跳过）。
fn model_path(data: &Path) -> Option<PathBuf> {
    std::env::var_os("QJ_MODEL")
        .map(PathBuf::from)
        .into_iter()
        .chain([
            data.join("models/hanzhang-tongbian/hanzhang-tongbian-small.qjm"),
            data.join("../models/hanzhang-tongbian/hanzhang-tongbian-small.qjm"),
        ])
        .find(|p| p.is_file())
}

/// 等状态到位。真模型加载加预热要几秒，放宽到两分钟。
fn wait_state(session: &mut Session, want: u8) {
    for _ in 0..(120_000 / 50) {
        if session.model_state() == want {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    panic!("模型状态没等到 {want}");
}

#[test]
fn load_unload_and_reload() {
    let Some(data) = data_dir() else {
        eprintln!("没有 QINGJIAN_DATA，跳过");
        return;
    };
    let Some(model) = model_path(&data) else {
        eprintln!("QINGJIAN_DATA 下与旁边都没找到通变模型，跳过");
        return;
    };
    let mut session = Session::open(&data, None, None, None).unwrap();
    assert_eq!(session.model_state(), MODEL_IDLE);
    assert!(session.load_model(&model, true));
    assert_eq!(session.model_state(), MODEL_LOADING);
    // 加载完成前会话照常出候选
    for c in "ni".chars() {
        session.push(c);
    }
    assert!(!session.entries().is_empty());
    session.clear();
    wait_state(&mut session, MODEL_ACTIVE);
    // 卸载后能再加载
    session.unload_model();
    assert_eq!(session.model_state(), MODEL_IDLE);
    assert!(session.load_model(&model, true));
    wait_state(&mut session, MODEL_ACTIVE);
}

#[test]
fn bad_path_fails_and_session_survives() {
    let Some(data) = data_dir() else {
        eprintln!("没有 QINGJIAN_DATA，跳过");
        return;
    };
    let mut session = Session::open(&data, None, None, None).unwrap();
    assert!(session.load_model(Path::new("/nonexistent/model.qjm"), true));
    wait_state(&mut session, MODEL_FAILED);
    for c in "ni".chars() {
        session.push(c);
    }
    assert!(!session.entries().is_empty());
}

/// 键盘的轮询拍子驱动重排：模型接上后打一句、停键，`poll` 在几拍内把神经分取回来并报候选栏要重画；
/// 键盘里不自由生成，候选里没有生成的那一类。
#[test]
fn poll_drives_rescoring_without_generation() {
    let Some(data) = data_dir() else {
        eprintln!("没有 QINGJIAN_DATA，跳过");
        return;
    };
    let Some(model) = model_path(&data) else {
        eprintln!("没找到通变模型，跳过");
        return;
    };
    let mut session = Session::open(&data, None, None, None).unwrap();
    assert!(session.load_model(&model, true));
    wait_state(&mut session, MODEL_ACTIVE);
    // 词图读不通的混输：开着生成时这里会出生成的候选
    for c in "zhegecanguandedianhuashishenme".chars() {
        session.push(c);
    }
    let mut redrawn = false;
    for _ in 0..40 {
        std::thread::sleep(std::time::Duration::from_millis(250));
        if session.poll() {
            redrawn = true;
            break;
        }
    }
    assert!(redrawn, "神经分一直没回来");
    assert!(!session.entries().is_empty());
    session.clear();
    for c in "woyongvscodexiedaima".chars() {
        session.push(c);
    }
    for _ in 0..8 {
        std::thread::sleep(std::time::Duration::from_millis(250));
        session.poll();
    }
    assert!(!session.entries().is_empty());
    assert!(
        session.entries().iter().all(|entry| !matches!(
            entry,
            Entry::Local(c) | Entry::Cloud(c) if c.kind == CandidateKind::Generated
        )),
        "键盘里不该有自由生成的整句"
    );
}
