//! 「记录中」的暂停：暂停只停「记」（输入日志），学习、提示、记一笔照常。
//! 与 `tests/session.rs` 同一套：`QINGJIAN_DATA` 指向含 `dict.qj` 的目录，没给就跳过。

mod support;

use qingjian_cloud_bridge::{CloudConfig, RecordingState, Session};

use self::support::data_dir;

/// 登录 + 开了「上传输入日志」的会话；服务器连不上不影响记日志（同 `tests/session.rs`）。
fn cloud_config(logs: bool) -> CloudConfig {
    toml::from_str::<CloudConfig>(&format!(
        "server = \"http://127.0.0.1:9\"\ntoken = \"t\"\nlogs = {logs}\n"
    ))
    .unwrap()
}

fn temp_user(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("qj-recording-{name}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// 打一句「你好」并上屏，再把日志刷到盘上。
fn type_a_line(session: &mut Session) {
    for c in "nihao".chars() {
        session.push(c);
    }
    session.commit(0);
    session.note_passthrough('\n');
    session.flush();
}

fn log_lines(user: &std::path::Path) -> usize {
    std::fs::read_to_string(user.join("input-log.jsonl"))
        .map(|text| text.lines().count())
        .unwrap_or(0)
}

#[test]
fn not_recording_without_the_cloud_switch() {
    let Some(data) = data_dir() else {
        return;
    };
    let user = temp_user("off");
    let now = 1_000;

    let mut session = Session::open(&data, Some(&user), None, Some(cloud_config(false))).unwrap();
    assert_eq!(session.recording_state(now), RecordingState::Off);
    // 没开开关就没有可暂停的东西：不写暂停文件
    session.pause_recording(Some(3600), now);
    assert!(!user.join("cloud/recording-pause.json").exists());

    type_a_line(&mut session);
    assert_eq!(log_lines(&user), 0, "没开开关就不记");

    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn pause_stops_the_log_and_expires() {
    let Some(data) = data_dir() else {
        return;
    };
    let user = temp_user("pause");
    let now = 1_000_000;

    let mut session = Session::open(&data, Some(&user), None, Some(cloud_config(true))).unwrap();
    assert_eq!(session.recording_state(now), RecordingState::Recording);
    type_a_line(&mut session);
    let before = log_lines(&user);
    assert!(before > 0, "开着的时候要记");

    // 定时暂停：这期间不再记
    session.pause_recording(Some(3600), now);
    assert_eq!(session.recording_state(now), RecordingState::PausedTimed);
    type_a_line(&mut session);
    assert_eq!(log_lines(&user), before, "暂停期间不再记");

    // 到点自动装回去
    assert_eq!(
        session.recording_state(now + 3600),
        RecordingState::Recording
    );
    type_a_line(&mut session);
    assert!(log_lines(&user) > before, "到点之后接着记");

    // 一直暂停，只有点恢复才回记录中
    session.pause_recording(None, now);
    assert_eq!(
        session.recording_state(now + 999_999),
        RecordingState::PausedForever
    );
    type_a_line(&mut session);
    let paused = log_lines(&user);
    session.resume_recording(now);
    assert_eq!(session.recording_state(now), RecordingState::Recording);
    type_a_line(&mut session);
    assert!(log_lines(&user) > paused, "恢复之后接着记");

    std::fs::remove_dir_all(&user).ok();
}

/// 键盘进程随时被杀：重开时要还记得在暂停（暂停文件在 App Group 里）。
#[test]
fn reopened_session_starts_paused() {
    let Some(data) = data_dir() else {
        return;
    };
    let user = temp_user("reopen");
    // 会话打开时按真实时钟判断暂停到没到，所以这里也必须用真实时间写暂停文件
    let now: i64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let mut session = Session::open(&data, Some(&user), None, Some(cloud_config(true))).unwrap();
    session.pause_recording(Some(3600), now);
    session.flush();
    drop(session);

    let mut again = Session::open(&data, Some(&user), None, Some(cloud_config(true))).unwrap();
    assert_eq!(again.recording_state(now), RecordingState::PausedTimed);
    type_a_line(&mut again);
    assert_eq!(log_lines(&user), 0, "重开的会话一开始就不记");

    std::fs::remove_dir_all(&user).ok();
}
