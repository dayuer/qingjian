//! 「记一笔」素材库：切段不丢字、2000 字节上限、200 条未整理上限、30 天清理、忘掉时一起删、旧目录为空，
//! 以及上传前替换对象名字与补传判断这两个纯函数。

use qingjian_cloud_proto::{MAX_MEMORY_TEXT_BYTES, MemoryKind};

use super::{contact, id, open_with_scenes, temp_dir};
use crate::memory::{
    CONTACT_PLACEHOLDER, CloudState, Consent, MAX_UNPROCESSED_MATERIALS, MaterialSource,
    MemoryError, UploadDecision, mask_contact_names, should_upload, split_note, unprocessed,
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
    let store = open_with_scenes(&user);
    store.put_contact(contact(1)).unwrap();
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
    let store = open_with_scenes(&user);
    store.put_contact(contact(1)).unwrap();
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
    let store = open_with_scenes(&user);
    store.put_contact(contact(1)).unwrap();
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
    let store = open_with_scenes(&user);
    store.put_contact(contact(1)).unwrap();
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
    assert!(matches!(
        error,
        MemoryError::MaterialLimit {
            remaining: 1,
            needed: 2
        }
    ));
    assert_eq!(error.code(), "material_limit");
    assert_eq!(
        error.message(),
        "这次有 2 条，这个人只剩 1 个空位，先去 App 里整理"
    );
    let json: serde_json::Value = serde_json::from_str(&error.to_json()).unwrap();
    assert_eq!(json["remaining"], 1);
    assert_eq!(json["needed"], 2);

    store
        .add_material(&id(1), "第 200 条", MaterialSource::Typed, 3)
        .unwrap();
    let full = store
        .add_material(&id(1), "第 201 条", MaterialSource::Typed, 4)
        .unwrap_err();
    assert!(matches!(
        full,
        MemoryError::MaterialLimit {
            remaining: 0,
            needed: 1
        }
    ));
    assert_eq!(full.message(), "这个人还有 200 条没整理，先去 App 里看看");
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
    let store = open_with_scenes(&user);
    store.put_contact(contact(1)).unwrap();
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
    let store = open_with_scenes(&user);
    store.put_contact(contact(1)).unwrap();
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
    let store = open_with_scenes(&user);
    store.put_contact(contact(1)).unwrap();
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
    let store = open_with_scenes(&user);
    store.put_contact(contact(1)).unwrap();
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
    let mut person = contact(1);
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

/// 「+ 记一条」的无主桶：不绑对象、与按对象的那份同格式，归人时挪进那个人的 materials.jsonl。

#[test]
fn unassigned_bucket_starts_empty() {
    let user = temp_dir("unassigned-empty");
    let store = open_with_scenes(&user);
    assert!(store.unassigned_materials(0).unwrap().is_empty());
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn unassigned_note_stores_and_assigns() {
    let user = temp_dir("unassigned-assign");
    let store = open_with_scenes(&user);
    store.put_contact(contact(1)).unwrap();

    let added = store
        .add_unassigned_material("周五晚上订了两个人的位子", MaterialSource::Typed, 100)
        .unwrap();
    assert_eq!(added.len(), 1);
    assert!(!added[0].uploaded && !added[0].processed);
    assert_eq!(store.unassigned_materials(100).unwrap().len(), 1);
    assert!(store.materials(&id(1), 100).unwrap().is_empty(), "还没归人");

    store
        .assign_material(&added[0].client_id, &id(1), 200)
        .unwrap();
    assert!(
        store.unassigned_materials(200).unwrap().is_empty(),
        "归完就从无主桶摘掉"
    );
    let target = store.materials(&id(1), 200).unwrap();
    assert_eq!(target.len(), 1);
    assert_eq!(target[0].text, "周五晚上订了两个人的位子");
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn assign_requires_a_known_contact_and_a_stored_material() {
    let user = temp_dir("unassigned-errors");
    let store = open_with_scenes(&user);
    store.put_contact(contact(1)).unwrap();
    let added = store
        .add_unassigned_material("一句原话", MaterialSource::Typed, 100)
        .unwrap();

    // 名单上没有的人
    assert!(matches!(
        store.assign_material(&added[0].client_id, &id(9), 100),
        Err(MemoryError::Invalid(_))
    ));
    // 无主桶里没有这条
    assert!(matches!(
        store.assign_material(&id(5000), &id(1), 100),
        Err(MemoryError::Invalid(_))
    ));
    // 两次都没成，素材还在无主桶里
    assert_eq!(store.unassigned_materials(100).unwrap().len(), 1);
    std::fs::remove_dir_all(&user).ok();
}

/// 回归：无主桶放在 `memory/` 根上，**不能**放进 `<伪对象 id>/`。
/// `write_snapshot` 会把名单上没有的人的目录连内容一起删，放对象目录里每次写快照都会把它清掉。
#[test]
fn unassigned_survives_a_snapshot_write() {
    let user = temp_dir("unassigned-snapshot");
    let store = open_with_scenes(&user);
    store.put_contact(contact(1)).unwrap();
    store
        .add_unassigned_material("别被写快照冲掉", MaterialSource::Typed, 100)
        .unwrap();

    let snapshot = store.snapshot().unwrap();
    store.write_snapshot(&snapshot).unwrap();

    assert_eq!(
        store.unassigned_materials(100).unwrap().len(),
        1,
        "写一次快照不该动无主桶"
    );
    std::fs::remove_dir_all(&user).ok();
}

#[test]
fn unassigned_honours_the_same_limit() {
    let user = temp_dir("unassigned-limit");
    let store = open_with_scenes(&user);
    // 每条一段，塞满 200 条
    for index in 0..MAX_UNPROCESSED_MATERIALS {
        store
            .add_unassigned_material(&format!("第 {index} 条"), MaterialSource::Typed, 100)
            .unwrap();
    }
    assert!(matches!(
        store.add_unassigned_material("挤不下了", MaterialSource::Typed, 100),
        Err(MemoryError::MaterialLimit {
            remaining: 0,
            needed: 1
        })
    ));
    std::fs::remove_dir_all(&user).ok();
}

#[cfg(unix)]
fn set_writable(path: &std::path::Path, writable: bool) {
    use std::os::unix::fs::PermissionsExt;
    let mode = if writable { 0o755 } else { 0o555 };
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode)).unwrap();
}

/// 写目标失败时素材还留在无主桶；写完目标、摘无主桶时失败，重试也不重复不丢（先写目标、按 client_id 去重）。
#[cfg(unix)]
#[test]
fn assign_never_loses_a_material_when_a_write_fails() {
    let user = temp_dir("unassigned-write-fails");
    let store = open_with_scenes(&user);
    store.put_contact(contact(1)).unwrap();
    let added = store
        .add_unassigned_material("周六下午三点在老地方见", MaterialSource::Typed, 100)
        .unwrap();
    let client_id = added[0].client_id.clone();
    let memory = user.join(crate::memory::MEMORY_DIR);
    let target_dir = memory.join(id(1));

    // 目标目录只读：写目标失败，无主桶里那条不动
    set_writable(&target_dir, false);
    assert!(store.assign_material(&client_id, &id(1), 200).is_err());
    set_writable(&target_dir, true);
    assert_eq!(
        store.unassigned_materials(200).unwrap().len(),
        1,
        "写目标失败，素材还在无主桶"
    );
    assert!(store.materials(&id(1), 200).unwrap().is_empty());

    // 记忆根目录只读：目标写成了，摘无主桶失败；两边各一份，不丢
    set_writable(&memory, false);
    assert!(store.assign_material(&client_id, &id(1), 200).is_err());
    set_writable(&memory, true);
    assert_eq!(store.materials(&id(1), 200).unwrap().len(), 1);
    assert_eq!(store.unassigned_materials(200).unwrap().len(), 1);

    // 重试：目标按 client_id 去重，不多出第二份；无主桶摘掉
    store.assign_material(&client_id, &id(1), 300).unwrap();
    assert_eq!(store.materials(&id(1), 300).unwrap().len(), 1, "重试不重复");
    assert!(store.unassigned_materials(300).unwrap().is_empty());
    std::fs::remove_dir_all(&user).ok();
}

/// 往满了的人名下「补上」和记一笔一样报 material_limit，素材留在无主桶。
#[test]
fn assign_respects_the_unprocessed_limit() {
    let user = temp_dir("assign-over-limit");
    let store = open_with_scenes(&user);
    store.put_contact(contact(1)).unwrap();
    for n in 0..MAX_UNPROCESSED_MATERIALS {
        store
            .add_material(&id(1), &format!("第 {n} 条"), MaterialSource::Typed, 1)
            .unwrap();
    }
    let added = store
        .add_unassigned_material("满了之后的一句", MaterialSource::Typed, 2)
        .unwrap();
    let error = store
        .assign_material(&added[0].client_id, &id(1), 3)
        .unwrap_err();
    assert!(matches!(
        error,
        MemoryError::MaterialLimit {
            remaining: 0,
            needed: 1
        }
    ));
    assert_eq!(error.code(), "material_limit");
    assert_eq!(
        store.unassigned_materials(3).unwrap().len(),
        1,
        "没挪成，还在无主桶"
    );
    assert_eq!(
        store.materials(&id(1), 3).unwrap().len(),
        MAX_UNPROCESSED_MATERIALS
    );
    std::fs::remove_dir_all(&user).ok();
}
