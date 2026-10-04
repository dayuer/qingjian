//! 素笺素材的规则层脱敏：把手机号、座机、证件号、卡号、邮箱、链接、详细地址换成占位符。
//! 无网络、无状态；客户端上传前与服务端复查用同一份代码和 `tests/vectors.jsonl`。

mod rule_counts;
mod rules;

pub use self::rule_counts::RuleCounts;

/// 按固定顺序跑完所有规则，返回脱敏后的文字与各类命中数。
///
/// 顺序是：链接、邮箱、地址、关键词证件号、关键词卡号、微信 / QQ 号、证件号、银行卡、座机、手机号。
/// - 链接最先：整段换掉，里面的数字（查询参数里的手机号）就不会再被别的规则单独命中；
/// - 邮箱在数字规则之前：`13812345678@qq.com` 是邮箱，不是手机号；
/// - 地址在数字规则之前：门牌与室号里的数字不去碰数字规则；
/// - 带关键词的号码（「身份证 …」「卡号 …」「微信 …」「QQ …」）在无关键词的校验规则之前：
///   关键词本身就是证据，校验位或 Luhn 不过也换；QQ 号最长 11 位，要抢在手机号规则前，不然 `QQ 13812345678` 会算成手机号；
/// - 证件号先于银行卡：18 位证件号有 10% 概率碰巧过 Luhn，要判成证件号；
/// - 银行卡先于手机号：`+86` 加手机号是 13 位，银行卡规则自己排除它，留给手机号规则；
/// - 座机与手机号互不重叠（座机 0 开头、手机号 1 开头），排在最后。
///
/// 占位符里没有数字、`@`、`://`、字母，也不含「路 / 街 / 道」和后面跟着数字的关键词，后面的规则不会再命中它们。
pub fn redact_rules(text: &str) -> (String, RuleCounts) {
    let (text, url) = rules::url::apply(text);
    let (text, email) = rules::email::apply(&text);
    let (text, address) = rules::address::apply(&text);
    let (text, keyword_id) = rules::keyword_id::apply(&text);
    let (text, keyword_bank) = rules::keyword_bank::apply(&text);
    let (text, account) = rules::account::apply(&text);
    let (text, id_card) = rules::id_card::apply(&text);
    let (text, bank_card) = rules::bank_card::apply(&text);
    let (text, landline) = rules::landline::apply(&text);
    let (text, phone) = rules::phone::apply(&text);
    let counts = RuleCounts {
        phone,
        landline,
        id_card: keyword_id + id_card,
        bank_card: keyword_bank + bank_card,
        email,
        url,
        address,
        account,
    };
    (text, counts)
}
