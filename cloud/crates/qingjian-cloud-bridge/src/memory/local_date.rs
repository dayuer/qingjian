//! 北京时间（UTC+8，没有夏令时）的日历日，算「还有几天」用。不为这点事引日期库：
//! Unix 秒加 8 小时按天取整；公历换算用 Howard Hinnant 的 days_from_civil / civil_from_days。

use std::fmt;

use super::now_unix;

const SECS_PER_DAY: i64 = 86_400;

const BEIJING_OFFSET_SECS: i64 = 8 * 3600;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocalDate {
    /// 1970-01-01 起的天数。
    days: i64,
}

impl LocalDate {
    pub fn from_unix(secs: i64) -> Self {
        Self {
            days: (secs + BEIJING_OFFSET_SECS).div_euclid(SECS_PER_DAY),
        }
    }

    pub fn today() -> Self {
        Self::from_unix(now_unix())
    }

    pub fn from_ymd(year: i64, month: u32, day: u32) -> Option<Self> {
        if !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
            return None;
        }
        Some(Self {
            days: days_from_civil(year, month, day),
        })
    }

    /// 只认 `YYYY-MM-DD`（`2026-1-5`、`2026/01/05` 都不认）。
    pub fn parse(text: &str) -> Option<Self> {
        let bytes = text.as_bytes();
        let shaped = bytes.len() == 10
            && bytes.iter().enumerate().all(|(i, b)| {
                if i == 4 || i == 7 {
                    *b == b'-'
                } else {
                    b.is_ascii_digit()
                }
            });
        if !shaped {
            return None;
        }
        let year = text[0..4].parse().ok()?;
        let month = text[5..7].parse().ok()?;
        let day = text[8..10].parse().ok()?;
        Self::from_ymd(year, month, day)
    }

    /// 从 `self` 到 `other` 还有几天，过去的是负数。
    pub fn days_until(self, other: Self) -> i64 {
        other.days - self.days
    }

    pub fn add_days(self, days: i64) -> Self {
        Self {
            days: self.days + days,
        }
    }

    pub fn ymd(self) -> (i64, u32, u32) {
        civil_from_days(self.days)
    }

    /// 按年重复的日子（生日、纪念日）：`self` 的月日落在 `today` 当天或之后最近的一次；
    /// 2 月 29 日在平年算 2 月 28 日。
    pub fn next_anniversary(self, today: Self) -> Self {
        let (_, month, day) = self.ymd();
        let (year, _, _) = today.ymd();
        let this_year = Self::anniversary_in(year, month, day);
        if this_year >= today {
            this_year
        } else {
            Self::anniversary_in(year + 1, month, day)
        }
    }

    fn anniversary_in(year: i64, month: u32, day: u32) -> Self {
        Self {
            days: days_from_civil(year, month, day.min(days_in_month(year, month))),
        }
    }
}

impl fmt::Display for LocalDate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (year, month, day) = self.ymd();
        write!(f, "{year:04}-{month:02}-{day:02}")
    }
}

fn is_leap(year: i64) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_in_month(year: i64, month: u32) -> u32 {
    match month {
        2 if is_leap(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn days_from_civil(year: i64, month: u32, day: u32) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let yoe = year.rem_euclid(400);
    let month = i64::from(month);
    let day = i64::from(day);
    let doy = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    (year, month as u32, day as u32)
}
