//! 「记一笔」素材库：切段不丢字、2000 字节上限、200 条未整理上限、30 天清理、忘掉时一起删、旧目录为空，
//! 以及上传前替换对象名字与补传判断这两个纯函数。

use qingjian_cloud_proto::{MAX_MEMORY_TEXT_BYTES, MemoryKind, Scene};

use super::{contact, id, temp_dir};
use crate::memory::{
    CONTACT_PLACEHOLDER, CloudState, Consent, MAX_UNPROCESSED_MATERIALS, MaterialSource,
    MemoryError, MemoryStore, UploadDecision, mask_contact_names, should_upload, split_note,
    unprocessed,
};

const DAY: i64 = 86_400;

/// 去掉空白后的全部字，用来核对切段前后一个字不丢。
fn letters(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

#[test]
fn short_text_is_one_piece_trimmed() {
    assert_eq!(split_note("  她不吃香菜\r\n"), vec!["她不吃香菜"]);
    assert!(split_note(" \n\n ").is_empty());
}

#[test]
fn exactly_the_limit_stays_whole() {
    let text = "a".repeat(MAX_MEMORY_TEXT_BYTES);
    assert_eq!(split_note(&text), vec![text.clone()]);
    assert_eq!(split_note(&format!("{text}b")).len(), 2);
}

#[test]
fn paragraphs_are_packed_and_nothing_is_lost() {
    // 每段 300 个汉字 = 900 字节，两段加空行 1802 字节装得下一条，三段装不下
    let paragraph = "字".repeat(300);
    let text = [paragraph.as_str(); 5].join("\n\n");
    let pieces = split_note(&text);
    assert_eq!(pieces.len(), 3);
    assert!(pieces.iter().all(|p| p.len() <= MAX_MEMORY_TEXT_BYTES));
    assert_eq!(
        pieces[0],
        format!("{paragraph}\n\n{paragraph}"),
        "段之间的空行留着"
    );
    assert_eq!(letters(&pieces.concat()), letters(&text));
}

#[test]
fn a_long_paragraph_is_cut_on_char_boundaries() {
    // 3 字节的汉字、4 字节的 emoji 混着，2000 不是它们的整数倍
    let text = "中😀".repeat(400);
    let pieces = split_note(&text);
    assert!(pieces.len() >= 2);
    for piece in &pieces {
        assert!(piece.len() <= MAX_MEMORY_TEXT_BYTES);
        assert!(piece.len() > MAX_MEMORY_TEXT_BYTES - 4 || piece == pieces.last().unwrap());
    }
    assert_eq!(pieces.concat(), text, "硬切不丢字、不切断字符");
}

#[test]
fn wechat_multi_copy_keeps_names_and_times() {
    let message = "阿杰\n2026年10月05日 09:34\n这一两个月业务还可以，一点点来。";
    let text = [message; 40].join("\n\n");
    let pieces = split_note(&text);
    assert!(pieces.len() > 1);
    assert!(pieces[0].starts_with("阿杰\n2026年10月05日 09:34"));
    assert_eq!(letters(&pieces.concat()), letters(&text));
}

#[test]
fn notes_go_to_materials_not_cards() {
    let user = temp_dir("materials-add");
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1, Scene::Dating)).unwrap();
    let added = store
        .add_material(&id(1), "  周末一起看电影 ", MaterialSource::Clipboard, 100)
        .unwrap();
    assert_eq!(added.len(), 1);
    let material = &added[0];
    assert_eq!(material.text, "周末一起看电影");
    assert_eq!(material.kind, MemoryKind::Note);
    assert_eq!(material.source, MaterialSource::Clipboard);
    assert_eq!(material.at, 100);
    assert!(!material.uploaded && !material.processed);
    assert_eq!(material.client_id.len(), 32);
    assert!(store.cards(&id(1)).is_empty(), "不再写卡");

    let line =
        std::fs::read_to_string(user.join("memory").join(id(1)).join("materials.jsonl")).unwrap();
    let json: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
    assert_eq!(json["kind"], "note");
    assert_eq!(json["source"], "clipboard");
    assert_eq!(json["uploaded"], false);
    assert!(json.get("processed_at").is_none());

    let long = "字".repeat(1000);
    let added = store
        .add_material(&id(1), &long, MaterialSource::Typed, 101)
        .unwrap();
    assert_eq!(added.len(), 2, "3000 字节切成两条");
    assert_eq!(store.materials(&id(1), 101).unwrap().len(), 3);
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn notes_need_a_contact_on_the_list_and_some_text() {
    let user = temp_dir("materials-reject");
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1, Scene::Daily)).unwrap();
    assert!(matches!(
        store.add_material(&id(2), "x", MaterialSource::Typed, 1),
        Err(MemoryError::Invalid(_))
    ));
    assert!(matches!(
        store.add_material(&id(1), "  ", MaterialSource::Typed, 1),
        Err(MemoryError::Invalid(_))
    ));
    assert!(matches!(
        store.add_material("../x", "x", MaterialSource::Typed, 1),
        Err(MemoryError::Invalid(_))
    ));
    assert!(!user.join("memory").join(id(2)).exists());
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn old_contact_dir_without_a_file_reads_empty() {
    let user = temp_dir("materials-empty");
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1, Scene::Dating)).unwrap();
    assert!(store.materials(&id(1), 1).unwrap().is_empty());
    assert!(
        store.materials(&id(9), 1).unwrap().is_empty(),
        "不在名单上的也为空"
    );
    assert!(!user.join("memory").join(id(9)).exists(), "读不建目录");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn unprocessed_limit_rejects_without_dropping() {
    let user = temp_dir("materials-limit");
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1, Scene::Dating)).unwrap();
    for n in 0..MAX_UNPROCESSED_MATERIALS - 1 {
        store
            .add_material(&id(1), &format!("第 {n} 条"), MaterialSource::Typed, 1)
            .unwrap();
    }
    // 199 条时一次切成两条的也整次拒绝，一条都不写
    let two = "字".repeat(1000);
    let error = store
        .add_material(&id(1), &two, MaterialSource::Clipboard, 2)
        .unwrap_err();
    assert!(matches!(error, MemoryError::MaterialLimit));
    assert_eq!(error.code(), "material_limit");
    assert_eq!(error.message(), "这个人还有 200 条没整理，先去 App 里看看");

    store
        .add_material(&id(1), "第 200 条", MaterialSource::Typed, 3)
        .unwrap();
    assert!(matches!(
        store.add_material(&id(1), "第 201 条", MaterialSource::Typed, 4),
        Err(MemoryError::MaterialLimit)
    ));
    let all = store.materials(&id(1), 4).unwrap();
    assert_eq!(unprocessed(&all), MAX_UNPROCESSED_MATERIALS);

    // 整理掉一条就又能记
    let first = all[0].client_id.clone();
    store
        .mark_materials_processed(&id(1), &[first.as_str()], 5)
        .unwrap();
    store
        .add_material(&id(1), "又能记了", MaterialSource::Typed, 6)
        .unwrap();
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn processed_materials_are_pruned_after_thirty_days() {
    let user = temp_dir("materials-prune");
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1, Scene::Dating)).unwrap();
    let a = store
        .add_material(&id(1), "早整理的", MaterialSource::Typed, 0)
        .unwrap()
        .remove(0);
    let b = store
        .add_material(&id(1), "晚整理的", MaterialSource::Typed, 0)
        .unwrap()
        .remove(0);
    store
        .add_material(&id(1), "没整理的", MaterialSource::Typed, 0)
        .unwrap();
    store
        .mark_materials_uploaded(&id(1), &[a.client_id.as_str(), b.client_id.as_str()], 1)
        .unwrap();
    store
        .mark_materials_processed(&id(1), &[a.client_id.as_str()], DAY)
        .unwrap();
    store
        .mark_materials_processed(&id(1), &[b.client_id.as_str()], 10 * DAY)
        .unwrap();
    let all = store.materials(&id(1), 20 * DAY).unwrap();
    assert_eq!(all.len(), 3);
    assert!(all[0].uploaded && all[0].processed);
    assert_eq!(all[0].processed_at, Some(DAY));

    // 读的时候顺手删：第 31 天早整理的那条没了，文件里也没了
    let texts: Vec<String> = store
        .materials(&id(1), 31 * DAY + 1)
        .unwrap()
        .into_iter()
        .map(|m| m.text)
        .collect();
    assert_eq!(texts, ["晚整理的", "没整理的"]);
    let file =
        std::fs::read_to_string(user.join("memory").join(id(1)).join("materials.jsonl")).unwrap();
    assert!(!file.contains("早整理的"));

    // 写的时候也删
    store
        .add_material(&id(1), "新记的", MaterialSource::Typed, 41 * DAY)
        .unwrap();
    let texts: Vec<String> = store
        .materials(&id(1), 41 * DAY)
        .unwrap()
        .into_iter()
        .map(|m| m.text)
        .collect();
    assert_eq!(texts, ["没整理的", "新记的"], "没整理的永远不删");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn delete_one_material() {
    let user = temp_dir("materials-delete");
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1, Scene::Dating)).unwrap();
    let a = store
        .add_material(&id(1), "删掉的", MaterialSource::Typed, 1)
        .unwrap()
        .remove(0);
    store
        .add_material(&id(1), "留着的", MaterialSource::Typed, 2)
        .unwrap();
    store.delete_material(&id(1), &a.client_id, 3).unwrap();
    store.delete_material(&id(1), &a.client_id, 3).unwrap();
    let texts: Vec<String> = store
        .materials(&id(1), 3)
        .unwrap()
        .into_iter()
        .map(|m| m.text)
        .collect();
    assert_eq!(texts, ["留着的"]);
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn forgetting_a_contact_deletes_its_materials() {
    let user = temp_dir("materials-forget");
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1, Scene::Dating)).unwrap();
    store
        .add_material(&id(1), "喜欢猫", MaterialSource::Typed, 1)
        .unwrap();
    let file = user.join("memory").join(id(1)).join("materials.jsonl");
    assert!(file.exists());
    store.forget_contact(&id(1)).unwrap();
    assert!(!file.exists());
    assert!(store.materials(&id(1), 2).unwrap().is_empty());
    assert!(matches!(
        store.add_material(&id(1), "又记一笔", MaterialSource::Typed, 2),
        Err(MemoryError::Invalid(_))
    ));
    assert!(
        !user.join("memory").join(id(1)).exists(),
        "目录没被重新建出来"
    );
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn a_corrupt_file_is_quarantined() {
    let user = temp_dir("materials-corrupt");
    let store = MemoryStore::open(&user);
    store.put_contact(contact(1, Scene::Dating)).unwrap();
    let dir = user.join("memory").join(id(1));
    std::fs::write(dir.join("materials.jsonl"), "{坏了\n").unwrap();
    assert!(store.materials(&id(1), 1).unwrap().is_empty());
    let backups = std::fs::read_dir(&dir)
        .unwrap()
        .filter(|entry| {
            entry
                .as_ref()
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with("materials.jsonl.broken-")
        })
        .count();
    assert_eq!(backups, 1);
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn contact_names_are_masked_longest_first() {
    let mut person = contact(1, Scene::Dating);
    person.name = "王小美".to_owned();
    person.display_name = Some("小美".to_owned());
    let text = "王小美\n2026年10月05日 09:34\n小美说周末去看海，阿杰也去";
    assert_eq!(
        mask_contact_names(text, &person),
        "〔对象〕\n2026年10月05日 09:34\n〔对象〕说周末去看海，阿杰也去",
        "其他人名留给服务端脱敏"
    );

    // 代号比名字长时也是长的先换
    person.name = "美".to_owned();
    person.display_name = Some("美美同学".to_owned());
    assert_eq!(
        mask_contact_names("美美同学和美", &person),
        "〔对象〕和〔对象〕"
    );

    // 代号为空白当没有；名字恰好是「对象」也不会在占位里再换一遍
    person.name = "对象".to_owned();
    person.display_name = Some("  ".to_owned());
    assert_eq!(mask_contact_names("对象来了", &person), "〔对象〕来了");
    assert_eq!(CONTACT_PLACEHOLDER, "〔对象〕");
    assert_eq!(mask_contact_names("没有名字", &person), "没有名字");
}

#[test]
fn upload_needs_cloud_and_consent() {
    use CloudState::{Activated, NotActivated};
    use Consent::{Given, NotGiven, Withdrawn};
    use UploadDecision::{Hold, Stop, Upload};
    let cases = [
        // 没开通：攒着
        (NotActivated, NotGiven, true, Hold),
        (NotActivated, NotGiven, false, Hold),
        (NotActivated, Given, true, Hold),
        (NotActivated, Given, false, Hold),
        (NotActivated, Withdrawn, true, Stop),
        (NotActivated, Withdrawn, false, Stop),
        // 开通未同意：继续留在本机
        (Activated, NotGiven, true, Hold),
        (Activated, NotGiven, false, Hold),
        // 开通且同意：有没传的才传
        (Activated, Given, true, Upload),
        (Activated, Given, false, Hold),
        // 同意后撤回：停传
        (Activated, Withdrawn, true, Stop),
        (Activated, Withdrawn, false, Stop),
    ];
    for (cloud, consent, pending, expected) in cases {
        assert_eq!(
            should_upload(cloud, consent, pending),
            expected,
            "{cloud:?} {consent:?} 有未传 {pending}"
        );
    }
}
