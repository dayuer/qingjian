//! 2C 记忆卡片的 JSON 形状：服务端与客户端按这里的字段名对接。

use qingjian_cloud_proto::{
    CardKind, CardPage, CardSource, MAX_CARD_KEYWORDS, MAX_CARD_PAGE, MAX_CARD_TEXT_CHARS,
    MemoryCard, PATH_MEMORY_CARDS, PutCard,
};
use serde_json::json;

fn full_card() -> serde_json::Value {
    json!({
        "card_id": "c1", "contact_id": "k1", "kind": "promise", "text": "答应周末去看展",
        "keywords": ["看展", "周末"], "when": "2026-10-10", "source": "cloud",
        "confirmed": false, "faded": false, "deleted": false, "seq": 7, "updated_at": 1_760_000_000_000_i64
    })
}

#[test]
fn card_round_trips_with_null_fields() {
    let card: MemoryCard = serde_json::from_value(full_card()).unwrap();
    assert_eq!(card.kind, CardKind::Promise);
    assert_eq!(card.source, CardSource::Cloud);
    assert_eq!(card.contact_id.as_deref(), Some("k1"));
    assert_eq!(card.when.as_deref(), Some("2026-10-10"));
    assert_eq!(card.keywords, ["看展", "周末"]);
    assert_eq!((card.seq, card.updated_at), (7, 1_760_000_000_000));
    assert_eq!(serde_json::to_value(&card).unwrap(), full_card());

    let unassigned = json!({
        "card_id": "c2", "contact_id": null, "kind": "other", "text": "她不吃香菜",
        "keywords": [], "when": null, "source": "manual",
        "confirmed": true, "faded": true, "deleted": true, "seq": 8, "updated_at": 5
    });
    let card: MemoryCard = serde_json::from_value(unassigned.clone()).unwrap();
    assert_eq!((card.contact_id.clone(), card.when.clone()), (None, None));
    assert!(card.confirmed && card.faded && card.deleted);
    assert_eq!(serde_json::to_value(&card).unwrap(), unassigned);
}

#[test]
fn missing_fields_take_defaults_and_unknown_fields_are_ignored() {
    let card: MemoryCard = serde_json::from_value(json!({
        "card_id": "c3", "kind": "date", "text": "生日", "source": "manual",
        "from_the_future": { "x": 1 }
    }))
    .unwrap();
    assert_eq!(card.contact_id, None);
    assert_eq!(card.when, None);
    assert!(card.keywords.is_empty());
    assert!(!card.confirmed && !card.faded && !card.deleted);
    assert_eq!((card.seq, card.updated_at), (0, 0));
    // card_id、kind、text、source 仍然必填
    assert!(serde_json::from_value::<MemoryCard>(json!({ "card_id": "c4" })).is_err());
}

#[test]
fn kind_and_source_are_lowercase() {
    for (kind, text) in [
        (CardKind::Date, "date"),
        (CardKind::Promise, "promise"),
        (CardKind::Preference, "preference"),
        (CardKind::Recent, "recent"),
        (CardKind::Other, "other"),
    ] {
        assert_eq!(serde_json::to_value(kind).unwrap(), json!(text));
        assert_eq!(
            serde_json::from_value::<CardKind>(json!(text)).unwrap(),
            kind
        );
    }
    assert_eq!(
        serde_json::to_value(CardSource::Cloud).unwrap(),
        json!("cloud")
    );
    assert_eq!(
        serde_json::to_value(CardSource::Manual).unwrap(),
        json!("manual")
    );
    assert!(serde_json::from_value::<CardKind>(json!("Date")).is_err());
}

#[test]
fn card_page_round_trips() {
    let value = json!({ "cards": [full_card()], "latest": 9 });
    let page: CardPage = serde_json::from_value(value.clone()).unwrap();
    assert_eq!((page.cards.len(), page.latest), (1, 9));
    assert_eq!(serde_json::to_value(&page).unwrap(), value);
}

#[test]
fn put_card_only_serializes_what_is_set() {
    assert_eq!(serde_json::to_value(PutCard::default()).unwrap(), json!({}));
    let confirm = PutCard {
        confirmed: Some(true),
        ..PutCard::default()
    };
    assert_eq!(
        serde_json::to_value(&confirm).unwrap(),
        json!({ "confirmed": true })
    );
    let clear = PutCard {
        clear_when: true,
        ..PutCard::default()
    };
    assert_eq!(
        serde_json::to_value(&clear).unwrap(),
        json!({ "clear_when": true })
    );
    let all = PutCard {
        kind: Some(CardKind::Date),
        text: Some("生日".to_owned()),
        keywords: Some(vec!["生日".to_owned()]),
        when: Some("2026-12-01".to_owned()),
        confirmed: Some(false),
        contact_id: Some("k1".to_owned()),
        clear_when: false,
    };
    let value = serde_json::to_value(&all).unwrap();
    assert_eq!(
        value,
        json!({ "kind": "date", "text": "生日", "keywords": ["生日"], "when": "2026-12-01",
                "confirmed": false, "contact_id": "k1" })
    );
    assert_eq!(serde_json::from_value::<PutCard>(value).unwrap(), all);
    // 只给 confirmed 的旧写法，其余按缺省
    let parsed: PutCard = serde_json::from_value(json!({ "confirmed": true })).unwrap();
    assert_eq!(parsed, confirm);
}

#[test]
fn path_and_page_limit() {
    assert_eq!(PATH_MEMORY_CARDS, "/v1/memory/cards");
    assert_eq!(MAX_CARD_PAGE, 500);
    assert_eq!(MAX_CARD_TEXT_CHARS, 200);
    assert_eq!(MAX_CARD_KEYWORDS, 8);
}
