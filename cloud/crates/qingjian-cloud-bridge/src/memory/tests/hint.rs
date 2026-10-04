//! 提示：两字以上的词才算、停用词不触发、排序、10 分钟节流、「知道了」当天不出、重建时带上节流与「知道了」、
//! 0–3 天的提醒（日子按年重复、约定只一次）与称呼、`more`、面板挑卡、最近 24 字。

use qingjian_cloud_proto::CardKind;

use super::{card, id};
use crate::memory::{
    HintIndex, HintReason, LocalDate, Pronoun, RECENT_CHARS, RecentText, panel_cards, reminder_text,
};

/// 2026-10-04 10:00 北京时间。
const NOW: i64 = 1_791_079_200;

/// 测试用的切词：卡片文字里用 `/` 标好词界。
fn split(text: &str) -> Vec<String> {
    text.split('/').map(str::to_owned).collect()
}

#[test]
fn matches_terms_of_two_or_more_chars() {
    let cards = [card(1, CardKind::Other, "想去/海边/看/日出", &[], None, 1)];
    let mut index = HintIndex::build(&cards, split);
    let hint = index.match_text("周末我们去海边吧", NOW).unwrap();
    assert_eq!(hint.card_id, id(1001));
    assert_eq!(hint.reason, HintReason::Match);
    assert_eq!(hint.text, "想去/海边/看/日出");
    assert!(!hint.more, "只有一张卡");
    assert!(index.match_text("今天天气不错", NOW + 1).is_none());
    assert!(index.match_text("看", NOW + 2000).is_none(), "单字不算词");
}

#[test]
fn keywords_count_and_stopwords_never_trigger() {
    let cards = [card(
        1,
        CardKind::Other,
        "我们/一起/散步",
        &["今天", " 公园 "],
        None,
        1,
    )];
    let mut index = HintIndex::build(&cards, split);
    assert!(index.match_text("我们今天一起吃饭", NOW).is_none());
    assert!(
        index.match_text("去公园吗", NOW + 1).is_some(),
        "关键词去掉首尾空白"
    );
    assert!(index.match_text("饭后去散步", NOW + 2).is_some());
}

#[test]
fn more_hits_then_newer_cards_rank_first() {
    let cards = [
        card(1, CardKind::Other, "海边", &[], None, 5),
        card(2, CardKind::Other, "海边/日出", &[], None, 1),
        card(3, CardKind::Other, "海边", &[], None, 9),
    ];
    let mut index = HintIndex::build(&cards, split);
    let hint = index.match_text("海边看日出", NOW).unwrap();
    assert_eq!(hint.card_id, id(1002));
    assert!(hint.more);
    let mut index = HintIndex::build(&cards, split);
    assert_eq!(
        index.match_text("去海边", NOW).unwrap().card_id,
        id(1003),
        "命中数一样时新改过的在前"
    );
}

#[test]
fn same_card_waits_ten_minutes_after_it_goes_away() {
    let cards = [card(1, CardKind::Other, "海边", &[], None, 1)];
    let mut index = HintIndex::build(&cards, split);
    assert!(index.match_text("海边", NOW).is_some());
    assert!(
        index.match_text("海边呀", NOW + 5).is_some(),
        "一直命中时接着显示"
    );
    assert!(index.match_text("吃饭", NOW + 10).is_none());
    assert!(
        index.match_text("海边", NOW + 60).is_none(),
        "消失后 10 分钟内不再出"
    );
    assert!(index.match_text("海边", NOW + 5 + 600).is_some());

    index.dismiss(&id(1001), false, NOW + 700);
    assert!(
        index.match_text("海边", NOW + 760).is_none(),
        "关掉也算出过"
    );
    assert!(index.match_text("海边", NOW + 1300).is_some());
}

#[test]
fn dismissed_today_stays_quiet_until_tomorrow() {
    let cards = [card(1, CardKind::Other, "海边", &[], None, 1)];
    let mut index = HintIndex::build(&cards, split);
    assert!(index.match_text("海边", NOW).is_some());
    index.dismiss(&id(1001), true, NOW);
    assert!(index.match_text("海边", NOW + 700).is_none());
    assert!(
        index.match_text("海边", NOW + 86_400).is_some(),
        "第二天照常"
    );
}

#[test]
fn reminders_cover_today_to_three_days() {
    let today = LocalDate::parse("2026-10-04").unwrap();
    let at = |when: &str| {
        let cards = [card(1, CardKind::Date, "生日", &[], Some(when), 1)];
        HintIndex::build(&cards, split)
            .today(today, Pronoun::Ta, "小美")
            .map(|hint| hint.text)
    };
    assert_eq!(at("2026-10-04").as_deref(), Some("今天是TA的生日"));
    assert_eq!(at("2026-10-05").as_deref(), Some("明天是TA的生日"));
    assert_eq!(at("2026-10-07").as_deref(), Some("3 天后是TA的生日"));
    assert_eq!(at("2026-10-08"), None);
    assert_eq!(at("2026-10-03"), None, "今年的过了，明年的还远");
    assert_eq!(
        at("1998-10-05").as_deref(),
        Some("明天是TA的生日"),
        "日子按年重复，写出生那年也行"
    );

    let cards = [
        card(1, CardKind::Preference, "生日", &[], Some("2026-10-04"), 1),
        card(2, CardKind::Promise, "看电影", &[], Some("2026-10-06"), 1),
        card(3, CardKind::Date, "纪念日", &[], Some("2026-10-05"), 1),
    ];
    let hint = HintIndex::build(&cards, split)
        .today(today, Pronoun::TaF, "小美")
        .unwrap();
    assert_eq!(hint.reason, HintReason::Today);
    assert_eq!(hint.card_id, id(1003), "只看日子与约定，近的先出");
    assert_eq!(hint.text, "明天是她的纪念日");
}

#[test]
fn dismissed_reminder_stays_quiet_today() {
    let today = LocalDate::from_unix(NOW);
    let cards = [card(1, CardKind::Date, "生日", &[], Some("2026-10-05"), 1)];
    let mut index = HintIndex::build(&cards, split);
    assert!(index.today(today, Pronoun::Ta, "小美").is_some());
    index.dismiss(&id(1001), true, NOW);
    assert!(index.today(today, Pronoun::Ta, "小美").is_none());
    assert!(
        index
            .today(today.add_days(1), Pronoun::Ta, "小美")
            .is_some()
    );
}

#[test]
fn pronouns_fill_the_template() {
    assert_eq!(
        reminder_text(CardKind::Date, 1, Pronoun::Ta, "小美", "生日"),
        "明天是TA的生日"
    );
    assert_eq!(
        reminder_text(CardKind::Date, 1, Pronoun::TaM, "小美", "生日"),
        "明天是他的生日"
    );
    assert_eq!(
        reminder_text(CardKind::Date, 1, Pronoun::TaF, "小美", "生日"),
        "明天是她的生日"
    );
    assert_eq!(
        reminder_text(CardKind::Date, 1, Pronoun::Name, "小美", "生日"),
        "明天是小美的生日"
    );
    assert_eq!(
        reminder_text(CardKind::Date, 0, Pronoun::Ta, "小美", "约会"),
        "今天是TA的约会"
    );
    assert_eq!(
        reminder_text(CardKind::Date, 2, Pronoun::Ta, "小美", "约会"),
        "2 天后是TA的约会"
    );
}

#[test]
fn promises_remind_once_with_their_own_template() {
    let today = LocalDate::parse("2026-10-04").unwrap();
    let at = |when: &str| {
        let cards = [card(1, CardKind::Promise, "看电影", &[], Some(when), 1)];
        HintIndex::build(&cards, split)
            .today(today, Pronoun::TaF, "小美")
            .map(|hint| hint.text)
    };
    assert_eq!(at("2026-10-05").as_deref(), Some("明天：看电影"));
    assert_eq!(at("2026-10-04").as_deref(), Some("今天：看电影"));
    assert_eq!(at("2026-10-07").as_deref(), Some("3 天后：看电影"));
    assert_eq!(at("2025-10-05"), None, "约定不按年重复");
}

#[test]
fn yearly_dates_cross_years_and_leap_days() {
    let remind = |when: &str, today: &str| {
        let cards = [card(1, CardKind::Date, "生日", &[], Some(when), 1)];
        HintIndex::build(&cards, split)
            .today(LocalDate::parse(today).unwrap(), Pronoun::Ta, "小美")
            .map(|hint| hint.text)
    };
    assert_eq!(
        remind("2020-01-02", "2026-12-30").as_deref(),
        Some("3 天后是TA的生日")
    );
    assert_eq!(
        remind("2024-02-29", "2026-02-27").as_deref(),
        Some("明天是TA的生日"),
        "平年按 2 月 28 日"
    );
    assert_eq!(
        remind("2024-02-29", "2028-02-28").as_deref(),
        Some("明天是TA的生日"),
        "闰年是 2 月 29 日"
    );
}

#[test]
fn more_means_another_card_besides_this_one() {
    let one = [card(1, CardKind::Other, "海边", &[], None, 1)];
    assert!(
        !HintIndex::build(&one, split)
            .match_text("海边", NOW)
            .unwrap()
            .more
    );
    let two = [
        card(1, CardKind::Other, "海边", &[], None, 1),
        card(2, CardKind::Other, "猫", &[], None, 1),
    ];
    assert!(
        HintIndex::build(&two, split)
            .match_text("海边", NOW)
            .unwrap()
            .more
    );
    let mut unconfirmed = card(2, CardKind::Other, "猫咪", &[], None, 1);
    unconfirmed.confirmed = false;
    let mut index = HintIndex::build(&[one[0].clone(), unconfirmed], split);
    assert!(index.match_text("猫咪", NOW).is_none(), "没确认的卡不提示");
    assert!(!index.match_text("海边", NOW).unwrap().more, "也不算别的卡");
}

#[test]
fn rebuild_keeps_throttle_and_dismissals() {
    let cards = [
        card(1, CardKind::Other, "海边", &[], None, 1),
        card(2, CardKind::Other, "日出", &[], None, 1),
    ];
    let mut index = HintIndex::build(&cards, split);
    assert!(index.match_text("海边", NOW).is_some());
    assert!(index.match_text("吃饭", NOW + 1).is_none());
    index.dismiss(&id(1002), true, NOW);
    let more = [
        cards[0].clone(),
        cards[1].clone(),
        card(3, CardKind::Other, "猫", &[], None, 1),
    ];
    let mut rebuilt = index.rebuild(&more, split);
    assert!(
        rebuilt.match_text("海边", NOW + 60).is_none(),
        "重建后节流还在"
    );
    assert!(
        rebuilt.match_text("日出", NOW + 60).is_none(),
        "重建后「知道了」还在"
    );
    assert_eq!(rebuilt.dismissed().len(), 1);
    let gone = rebuilt.rebuild(&more[2..], split);
    let mut fresh = gone.rebuild(&more, split);
    assert!(
        fresh.match_text("海边", NOW + 60).is_some(),
        "卡不在了，节流记录跟着丢"
    );
}

#[test]
fn panel_puts_upcoming_dates_then_the_hinted_card_first() {
    let today = LocalDate::parse("2026-10-04").unwrap();
    let cards = vec![
        card(1, CardKind::Other, "猫叫团子", &[], None, 9),
        card(2, CardKind::Promise, "看电影", &[], Some("2026-10-06"), 1),
        card(3, CardKind::Date, "生日", &[], Some("2026-10-05"), 1),
        card(4, CardKind::Recent, "在准备考试", &[], None, 5),
        card(5, CardKind::Date, "纪念日", &[], Some("2026-12-01"), 20),
    ];
    let focus = id(1004);
    let picked: Vec<String> = panel_cards(cards, today, Some(focus.as_str()))
        .into_iter()
        .map(|card| card.id)
        .collect();
    assert_eq!(picked, vec![id(1003), id(1002), id(1004)]);
}

#[test]
fn recent_text_keeps_the_last_chars() {
    let mut recent = RecentText::default();
    recent.push_str(&"一".repeat(30));
    recent.push_str("海边");
    assert_eq!(recent.text().chars().count(), RECENT_CHARS);
    assert!(recent.text().ends_with("海边"));
    recent.clear();
    assert!(recent.text().is_empty());
}
