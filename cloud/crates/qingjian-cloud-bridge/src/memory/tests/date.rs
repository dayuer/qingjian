//! 北京时间的日历日：Unix 秒加 8 小时、`YYYY-MM-DD` 的解析与往返、跨月跨年的天数。

use crate::memory::LocalDate;

#[test]
fn unix_seconds_use_beijing_time() {
    assert_eq!(LocalDate::from_unix(0).to_string(), "1970-01-01");
    assert_eq!(
        LocalDate::from_unix(16 * 3600 - 1).to_string(),
        "1970-01-01"
    );
    assert_eq!(LocalDate::from_unix(16 * 3600).to_string(), "1970-01-02");
    // 2026-10-04 00:00 北京时间 = 2026-10-03T16:00Z
    assert_eq!(
        LocalDate::from_unix(1_791_043_200).to_string(),
        "2026-10-04"
    );
    assert_eq!(
        LocalDate::from_unix(1_791_043_199).to_string(),
        "2026-10-03"
    );
}

#[test]
fn parses_only_full_dates() {
    assert!(LocalDate::parse("2024-02-29").is_some());
    assert!(LocalDate::parse("2023-02-29").is_none());
    assert!(LocalDate::parse("2026-13-01").is_none());
    assert!(LocalDate::parse("2026-1-05").is_none());
    assert!(LocalDate::parse("2026/01/05").is_none());
    assert!(LocalDate::parse("+026-01-05").is_none());
    assert!(LocalDate::parse("").is_none());
}

#[test]
fn counts_days_across_months_and_years() {
    let day = |text: &str| LocalDate::parse(text).unwrap();
    assert_eq!(day("2025-12-31").days_until(day("2026-01-01")), 1);
    assert_eq!(day("2026-03-01").days_until(day("2026-02-28")), -1);
    assert_eq!(day("2024-02-28").add_days(1).to_string(), "2024-02-29");
    assert_eq!(day("2026-10-04").add_days(3).to_string(), "2026-10-07");
}

#[test]
fn round_trips_through_text() {
    let epoch = LocalDate::from_unix(0);
    for offset in -700_000..700_000 {
        if offset % 997 != 0 {
            continue;
        }
        let date = epoch.add_days(offset);
        assert_eq!(LocalDate::parse(&date.to_string()), Some(date), "{date}");
    }
}

#[test]
fn anniversaries_repeat_every_year() {
    let day = |text: &str| LocalDate::parse(text).unwrap();
    let next = |when: &str, today: &str| day(when).next_anniversary(day(today)).to_string();
    assert_eq!(next("1998-10-05", "2026-10-04"), "2026-10-05");
    assert_eq!(next("1998-10-04", "2026-10-04"), "2026-10-04", "当天算今年");
    assert_eq!(next("1998-10-03", "2026-10-04"), "2027-10-03", "过了看明年");
    assert_eq!(
        next("2020-01-02", "2026-12-30"),
        "2027-01-02",
        "12 月 30 日看 1 月 2 日"
    );
    assert_eq!(
        next("2024-02-29", "2026-02-27"),
        "2026-02-28",
        "平年按 2 月 28 日"
    );
    assert_eq!(next("2024-02-29", "2026-03-01"), "2027-02-28");
    assert_eq!(next("2024-02-29", "2028-02-28"), "2028-02-29", "闰年照常");
}
