//! 详细地址：省 / 市 / 区 / 县 起头，经街道到门牌「号」，后面的栋、单元、室一并带走；只到「路」不到「号」的不算。
//! 起头的行政区名前面常夹着「我住在」之类，正则会连带吃进去，所以匹配后再剥掉开头的虚词。

use std::sync::LazyLock;

use regex::Regex;

const PLACEHOLDER: &str = "〔地址〕";

/// 命中后可以从地址开头剥掉的字（人称、介词、动词）。
const LEADING_FILLER: &str =
    "我你他她它们在于到去来往从至是住的和与及送寄回家有要想把给让被将就也还都又再才刚正天";

static ADDRESS: LazyLock<Regex> = LazyLock::new(|| {
    let province = "(?:北京|上海|天津|重庆)市|(?:河北|山西|辽宁|吉林|黑龙江|江苏|浙江|安徽|福建|江西|山东|河南|湖北|湖南|广东|海南|四川|贵州|云南|陕西|甘肃|青海|台湾)省|(?:内蒙古|广西壮族|西藏|宁夏回族|新疆维吾尔)自治区";
    let first = format!(r"(?:{province}|\p{{Han}}{{2,4}}市|\p{{Han}}{{2,3}}(?:区|县))");
    let more = r"(?:\p{Han}{1,5}(?:市|区|县|旗|州))*";
    let town = r"(?:\p{Han}{1,6}(?:镇|乡|街道|村))?";
    let street = r"[\p{Han}A-Za-z0-9]{1,12}(?:路|街|道|巷|弄)";
    let number = r"\d{1,5}(?:-\d{1,5})?号";
    let detail = r"(?:\d{1,4}(?:号楼|栋|幢|楼|单元|室|层|座))*";
    Regex::new(&format!("{first}{more}{town}{street}{number}{detail}")).expect("地址正则")
});

pub fn apply(text: &str) -> (String, u32) {
    let mut output = String::with_capacity(text.len());
    let mut copied = 0;
    let mut count = 0;
    for found in ADDRESS.find_iter(text) {
        let matched = found.as_str();
        let kept = matched.trim_start_matches(|c| LEADING_FILLER.contains(c));
        let start = found.end() - kept.len();
        output.push_str(&text[copied..start]);
        output.push_str(PLACEHOLDER);
        copied = found.end();
        count += 1;
    }
    output.push_str(&text[copied..]);
    (output, count)
}
