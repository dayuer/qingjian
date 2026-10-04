//! 2B 素材上传的 JSON 形状：服务端与客户端按这里的字段名对接。

use qingjian_cloud_proto::{
    Consents, ContactRegistration, Feature, MAX_MEMORY_ITEMS, MAX_MEMORY_TEXT_BYTES,
    MemoryAccepted, MemoryItem, MemoryKind, MemoryPush, PATH_MEMORY_CONTACTS,
    PATH_MEMORY_MATERIALS, PATH_MEMORY_PROCESSOR, ProcessorInfo, Scene,
};
use serde_json::json;

#[test]
fn scene_and_kind_are_lowercase() {
    assert_eq!(serde_json::to_value(Scene::Daily).unwrap(), json!("daily"));
    assert_eq!(
        serde_json::to_value(Scene::Dating).unwrap(),
        json!("dating")
    );
    assert_eq!(serde_json::to_value(Scene::Work).unwrap(), json!("work"));
    assert_eq!(
        serde_json::from_value::<Scene>(json!("work")).unwrap(),
        Scene::Work
    );
    assert!(serde_json::from_value::<Scene>(json!("Work")).is_err());
    assert_eq!(
        serde_json::to_value(MemoryKind::Sent).unwrap(),
        json!("sent")
    );
    assert_eq!(
        serde_json::to_value(MemoryKind::Note).unwrap(),
        json!("note")
    );
    assert_eq!(
        serde_json::from_value::<MemoryKind>(json!("note")).unwrap(),
        MemoryKind::Note
    );
}

#[test]
fn memory_push_matches_the_spec_example() {
    let value = json!({
        "items": [
            { "client_id": "c1", "contact_id": "k1", "scene": "dating", "kind": "sent",
              "text": "晚上一起吃饭吗", "at": 1_760_000_000 },
            { "client_id": "c2", "contact_id": null, "scene": "daily", "kind": "note",
              "text": "她不吃香菜", "at": 1_760_000_100 }
        ]
    });
    let push: MemoryPush = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(push.items.len(), 2);
    assert_eq!(
        push.items[0],
        MemoryItem {
            client_id: "c1".to_owned(),
            contact_id: Some("k1".to_owned()),
            scene: Scene::Dating,
            kind: MemoryKind::Sent,
            text: "晚上一起吃饭吗".to_owned(),
            at: 1_760_000_000,
        }
    );
    assert_eq!(push.items[1].contact_id, None);
    assert_eq!(serde_json::to_value(&push).unwrap(), value);
}

#[test]
fn responses_and_registration_round_trip() {
    let accepted: MemoryAccepted = serde_json::from_value(json!({ "accepted": 3 })).unwrap();
    assert_eq!(accepted.accepted, 3);
    assert_eq!(
        serde_json::to_value(&accepted).unwrap(),
        json!({ "accepted": 3 })
    );

    let registration: ContactRegistration =
        serde_json::from_value(json!({ "scene": "dating" })).unwrap();
    assert_eq!(registration.scene, Scene::Dating);
    assert_eq!(
        serde_json::to_value(&registration).unwrap(),
        json!({ "scene": "dating" })
    );

    let processor: ProcessorInfo =
        serde_json::from_value(json!({ "name": "某供应商", "zero_retention": true })).unwrap();
    assert_eq!(processor.name, "某供应商");
    assert!(processor.zero_retention);
    assert_eq!(
        serde_json::to_value(&processor).unwrap(),
        json!({ "name": "某供应商", "zero_retention": true })
    );
}

#[test]
fn paths_and_limits() {
    assert_eq!(PATH_MEMORY_MATERIALS, "/v1/memory/materials");
    assert_eq!(PATH_MEMORY_CONTACTS, "/v1/memory/contacts");
    assert_eq!(PATH_MEMORY_PROCESSOR, "/v1/memory/processor");
    assert_eq!(MAX_MEMORY_ITEMS, 100);
    assert_eq!(MAX_MEMORY_TEXT_BYTES, 2000);
}

#[test]
fn memory_feature_and_consent() {
    assert_eq!(Feature::parse("memory"), Some(Feature::Memory));
    assert_eq!(Feature::Memory.as_str(), "memory");
    assert_eq!(
        serde_json::to_value(Feature::Memory).unwrap(),
        json!("memory")
    );
    assert_eq!(Feature::ALL.len(), 5);
    assert!(Feature::ALL.contains(&Feature::Memory));

    // 旧服务端 / 旧文件里没有 memory：按关
    let consents: Consents = serde_json::from_value(json!({
        "clipboard": true, "sync": true, "input_log": false, "llm": true
    }))
    .unwrap();
    assert!(!consents.memory && !consents.get(Feature::Memory));

    let mut consents = Consents::default();
    consents.set(Feature::Memory, true);
    assert!(consents.memory && consents.get(Feature::Memory));
    assert_eq!(
        serde_json::to_value(consents).unwrap()["memory"],
        json!(true)
    );
}
