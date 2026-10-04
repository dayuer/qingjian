//! 逐行跑 `vectors.jsonl`：服务端用路径依赖复用同一份文件，格式不要改。
//! 每行 `{"input","expected","counts"}`，`counts` 的九个键（phone / landline / id_card / bank_card / email / url / address / account / credential）都在。

use qingjian_cloud_redact::{RuleCounts, redact_rules};
use serde_json::Value;

fn counts_of(value: &Value) -> RuleCounts {
    let get = |key: &str| value[key].as_u64().unwrap_or_else(|| panic!("缺 {key}")) as u32;
    RuleCounts {
        phone: get("phone"),
        landline: get("landline"),
        id_card: get("id_card"),
        bank_card: get("bank_card"),
        email: get("email"),
        url: get("url"),
        address: get("address"),
        account: get("account"),
        credential: get("credential"),
    }
}

#[test]
fn vectors_match() {
    let text = include_str!("vectors.jsonl");
    let mut total = 0;
    let mut failures = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let row: Value =
            serde_json::from_str(line).unwrap_or_else(|e| panic!("第 {} 行：{e}", index + 1));
        let input = row["input"].as_str().unwrap();
        let expected = row["expected"].as_str().unwrap();
        let counts = counts_of(&row["counts"]);
        let (output, got) = redact_rules(input);
        if output != expected || got != counts {
            failures.push(format!(
                "第 {} 行 {input:?}\n  期望 {expected:?} {counts:?}\n  实际 {output:?} {got:?}",
                index + 1
            ));
        }
        total += 1;
    }
    assert!(total >= 80, "向量只有 {total} 条");
    assert!(
        failures.is_empty(),
        "{} 条不符：\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn redaction_is_idempotent_on_the_vector_outputs() {
    for line in include_str!("vectors.jsonl").lines() {
        let row: Value = serde_json::from_str(line).unwrap();
        let expected = row["expected"].as_str().unwrap();
        let (again, counts) = redact_rules(expected);
        assert_eq!(again, expected, "占位符不能被二次替换");
        assert_eq!(counts, RuleCounts::default());
    }
}
