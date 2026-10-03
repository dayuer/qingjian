//! 输入日志：上传去重、断点续传、下载别的设备的、清空在各设备间传播。

mod common;

use std::io::Write;
use std::path::{Path, PathBuf};

use qingjian_cloud_client::{Client, InputLogSync};

use common::TestServer;

struct Device {
    _dir: tempfile::TempDir,
    ime_dir: PathBuf,
    downloads: PathBuf,
    sync: InputLogSync,
}

impl Device {
    fn new(server: &TestServer, name: &str) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let ime_dir = dir.path().join("Qingjian");
        std::fs::create_dir_all(&ime_dir).unwrap();
        let downloads = dir.path().join("cloud/input-log");
        let client = Client::new(&server.url, &server.store.add_device(name).unwrap());
        let sync = InputLogSync::open(
            client,
            &ime_dir,
            &dir.path().join("cloud"),
            Some(downloads.clone()),
        )
        .unwrap();
        Self {
            _dir: dir,
            ime_dir,
            downloads,
            sync,
        }
    }

    fn log(&self, text: &str) {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.ime_dir.join("input-log.jsonl"))
            .unwrap();
        file.write_all(text.as_bytes()).unwrap();
    }

    fn clear(&self) {
        std::fs::File::create(self.ime_dir.join("input-log.jsonl")).unwrap();
    }
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

#[test]
fn uploads_complete_lines_once_and_others_download_them() {
    let server = TestServer::start();
    let mut mac = Device::new(&server, "mac");
    let mut phone = Device::new(&server, "iphone");

    mac.log("{\"event\":\"commit\",\"text\":\"一\"}\n{\"event\":\"commit\",\"text\":\"二\"}\n{\"event\":\"comm");
    assert_eq!(mac.sync.cycle().unwrap().uploaded, 2);
    // 半行等写完才传
    mac.log("it\",\"text\":\"三\"}\n");
    assert_eq!(mac.sync.cycle().unwrap().uploaded, 1);
    assert_eq!(mac.sync.cycle().unwrap().uploaded, 0);

    phone.log("{\"event\":\"commit\",\"text\":\"手机\"}\n");
    assert_eq!(phone.sync.cycle().unwrap().downloaded, 3);
    assert_eq!(read(&phone.downloads.join("mac.jsonl")).lines().count(), 3);
    assert!(read(&phone.downloads.join("mac.jsonl")).contains("三"));
    // 自己的不下载
    assert!(!phone.downloads.join("iphone.jsonl").exists());
    assert_eq!(mac.sync.cycle().unwrap().downloaded, 1);
    assert_eq!(server.store.input_log_since(0, 100).unwrap().lines.len(), 4);
}

#[test]
fn clearing_on_one_device_clears_server_and_other_copies() {
    let server = TestServer::start();
    let mut mac = Device::new(&server, "mac");
    let mut phone = Device::new(&server, "iphone");
    mac.log("{\"event\":\"commit\",\"text\":\"旧的内容很长很长\"}\n");
    mac.sync.cycle().unwrap();
    phone.sync.cycle().unwrap();
    assert!(phone.downloads.join("mac.jsonl").exists());

    // 清空后又打了几个字（文件不一定变短）
    mac.clear();
    mac.log("{\"event\":\"commit\",\"text\":\"新\"}\n{\"event\":\"commit\",\"text\":\"又一条新的内容\"}\n");
    let outcome = mac.sync.cycle().unwrap();
    assert!(outcome.cleared);
    assert_eq!(outcome.uploaded, 2);
    let lines = server.store.input_log_since(0, 100).unwrap().lines;
    assert_eq!(lines.len(), 2);
    assert!(lines[0].line.contains("新"));

    phone.sync.cycle().unwrap();
    let copy = read(&phone.downloads.join("mac.jsonl"));
    assert!(!copy.contains("旧的"));
    assert_eq!(copy.lines().count(), 2);
}

#[test]
fn retried_batches_are_not_duplicated() {
    let server = TestServer::start();
    let token = server.store.add_device("mac").unwrap();
    let client = Client::new(&server.url, &token);
    let push = qingjian_cloud_proto::InputLogPush {
        batch_id: "0:0".to_owned(),
        lines: vec!["{}".to_owned(), "{}".to_owned()],
    };
    client.push_input_log(&push).unwrap();
    client.push_input_log(&push).unwrap();
    assert_eq!(server.store.input_log_since(0, 100).unwrap().lines.len(), 2);
}
