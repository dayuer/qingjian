//! `export-log`：按 `seq` 顺序导出汇总的输入日志，一行一条原文。

use std::io::Write;
use std::path::Path;

use qingjian_cloud_proto::MAX_PAGE;

use crate::{ServerError, Store};

pub fn run(store: &Store, out: Option<&Path>, device: Option<&str>) -> Result<(), ServerError> {
    let mut writer: Box<dyn Write> = match out {
        Some(path) => Box::new(std::io::BufWriter::new(std::fs::File::create(path)?)),
        None => Box::new(std::io::stdout().lock()),
    };
    let mut since = 0;
    let mut count = 0;
    loop {
        let page = store.input_log_since(since, MAX_PAGE)?;
        let Some(last) = page.lines.last() else {
            break;
        };
        since = last.seq;
        for line in &page.lines {
            if device.is_none_or(|name| name == line.device) {
                writeln!(writer, "{}", line.line)?;
                count += 1;
            }
        }
    }
    writer.flush()?;
    if out.is_some() {
        println!("导出了 {count} 行");
    }
    Ok(())
}
