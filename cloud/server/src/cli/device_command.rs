//! `device add|list|remove`。

use clap::Subcommand;

use crate::{ServerError, Store};

#[derive(Debug, Subcommand)]
pub enum DeviceCommand {
    /// 登记一台设备并打印它的令牌（只显示这一次）。
    Add { name: String },

    /// 列出已登记的设备。
    List,

    /// 移除设备，令牌立即失效。
    Remove { name: String },
}

impl DeviceCommand {
    pub fn run(self, store: &Store) -> Result<(), ServerError> {
        match self {
            Self::Add { name } => {
                let token = store.add_device(&name)?;
                println!(
                    "设备 {} 的令牌（只显示这一次，填进客户端配置）：",
                    name.trim()
                );
                println!("{token}");
            }
            Self::List => {
                for device in store.list_devices()? {
                    let seen = device
                        .last_seen
                        .map_or_else(|| "从未使用".to_owned(), format_time);
                    println!(
                        "{}\t登记于 {}\t最近使用 {}",
                        device.name,
                        format_time(device.created_at),
                        seen
                    );
                }
            }
            Self::Remove { name } => {
                store.remove_device(&name)?;
                println!("已移除 {}", name.trim());
            }
        }
        Ok(())
    }
}

/// Unix 毫秒 → `2026-10-03 10:02 UTC`，不为这一处引入时间库。
fn format_time(ms: i64) -> String {
    let secs = ms.div_euclid(1000);
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02} UTC",
        rem / 3600,
        rem % 3600 / 60
    )
}

/// Howard Hinnant 的 days → 公历日期算法。
fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_utc_time() {
        assert_eq!(format_time(0), "1970-01-01 00:00 UTC");
        assert_eq!(format_time(1_700_000_000_000), "2023-11-14 22:13 UTC");
        assert_eq!(format_time(951_782_400_000), "2000-02-29 00:00 UTC");
    }
}
