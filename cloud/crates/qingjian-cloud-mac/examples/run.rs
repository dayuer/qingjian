//! 不装输入法跑一遍同步模块：`cargo run -p qingjian-cloud-mac --example run -- [秒数]`，用本机的
//! `QingjianCloud/config.toml`。跑够秒数后打印子菜单的内容。只给开发时验证定时器、连接与菜单行用。

#[cfg(target_os = "macos")]
fn main() {
    use objc2::MainThreadMarker;
    use objc2_foundation::{NSDate, NSRunLoop};

    let seconds: f64 = std::env::args()
        .nth(1)
        .and_then(|arg| arg.parse().ok())
        .unwrap_or(5.0);
    let mtm = MainThreadMarker::new().expect("main thread");
    qingjian_cloud_mac::start(mtm);
    let until = NSDate::dateWithTimeIntervalSinceNow(seconds);
    NSRunLoop::currentRunLoop().runUntilDate(&until);
    println!("revision {}", qingjian_cloud_mac::menu_revision());
    for line in qingjian_cloud_mac::menu_lines() {
        println!("{line:?}");
    }
}

#[cfg(not(target_os = "macos"))]
fn main() {}
